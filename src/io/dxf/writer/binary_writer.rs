//! Binary DXF writer

use super::stream_writer::DxfStreamWriter;
use crate::error::Result;
use crate::types::Handle;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// Binary DXF sentinel
const BINARY_DXF_SENTINEL: &[u8] = b"AutoCAD Binary DXF\r\n\x1a\x00";

/// Byte widths of an integer value in binary DXF.
#[derive(Clone, Copy)]
enum IntWidth {
    One,
    Two,
    Four,
    Eight,
}

/// Binary DXF stream writer
pub struct DxfBinaryWriter<W: Write> {
    writer: W,
    /// R12 files give a group code one byte (255 then two bytes for the
    /// extended-data range); R13 and later always use two.
    short_codes: bool,
    /// Stack buffer for formatting handles without heap allocation.
    hex_buf: [u8; 17],
}

impl<W: Write> DxfBinaryWriter<W> {
    /// Create a new binary DXF writer
    pub fn new(mut writer: W) -> Result<Self> {
        // Write the binary sentinel at the start
        writer.write_all(BINARY_DXF_SENTINEL)?;
        Ok(Self {
            writer,
            short_codes: false,
            hex_buf: [0u8; 17],
        })
    }

    /// A writer for the R12 binary layout, which has one-byte group codes.
    pub fn new_r12(writer: W) -> Result<Self> {
        let mut w = Self::new(writer)?;
        w.short_codes = true;
        Ok(w)
    }

    /// Write a DXF code as 16-bit little-endian
    fn write_code(&mut self, code: i32) -> Result<()> {
        if self.short_codes {
            if (0..255).contains(&code) {
                self.writer.write_u8(code as u8)?;
            } else {
                self.writer.write_u8(255)?;
                self.writer.write_i16::<LittleEndian>(code as i16)?;
            }
            return Ok(());
        }
        self.writer.write_i16::<LittleEndian>(code as i16)?;
        Ok(())
    }

    /// Write a null-terminated string
    fn write_null_string(&mut self, value: &str) -> Result<()> {
        self.writer.write_all(value.as_bytes())?;
        self.writer.write_u8(0)?;
        Ok(())
    }

    /// Preserved DXF pairs store their values as text, but their binary
    /// representation still follows the group code's numeric or chunk type.
    /// Return false only for values that retain the null-string layout.
    fn write_typed_text(&mut self, code: i32, value: &str) -> Result<bool> {
        use crate::error::DxfError;
        use crate::io::dxf::GroupCodeValueType;

        let invalid = || DxfError::Parse(format!("Invalid text value for binary DXF group {code}"));
        let text = value.trim();
        match GroupCodeValueType::from_raw_code(code) {
            GroupCodeValueType::String | GroupCodeValueType::Handle => return Ok(false),
            GroupCodeValueType::Bool => {
                self.write_bool(code, text.parse::<i64>().map_err(|_| invalid())? != 0)?
            }
            GroupCodeValueType::Byte | GroupCodeValueType::Int16 => {
                self.write_i16(code, text.parse().map_err(|_| invalid())?)?
            }
            GroupCodeValueType::Int32 => {
                self.write_i32(code, text.parse().map_err(|_| invalid())?)?
            }
            GroupCodeValueType::Int64 => {
                self.write_i64(code, text.parse().map_err(|_| invalid())?)?
            }
            GroupCodeValueType::Double | GroupCodeValueType::Point3D => {
                self.write_double(code, text.parse().map_err(|_| invalid())?)?
            }
            GroupCodeValueType::BinaryData => {
                if text.len() % 2 != 0 {
                    return Err(invalid());
                }
                let data = text
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| {
                        let high = (pair[0] as char).to_digit(16).ok_or_else(invalid)?;
                        let low = (pair[1] as char).to_digit(16).ok_or_else(invalid)?;
                        Ok(((high << 4) | low) as u8)
                    })
                    .collect::<Result<Vec<_>>>()?;
                self.write_binary(code, &data)?;
            }
            GroupCodeValueType::None => return Err(DxfError::InvalidDxfCode(code)),
        }
        Ok(true)
    }

    /// Write an integer at the width the group code fixes in binary DXF.
    ///
    /// The format gives every code its own width (16 bits for 60-79, 32 for
    /// 90-99, one byte for 290-299, and so on), whatever integer type the
    /// caller happens to hold. A writer that takes the width from the Rust type
    /// shifts every later group, which is what made readers refuse the file
    /// (acadsharp-rs-tests#110). A value that does not fit its group is an
    /// error: writing half of it would corrupt the file without saying so. A
    /// code the format does not define keeps the width of the caller's type.
    fn write_integer(&mut self, code: i32, value: i64, native: IntWidth) -> Result<()> {
        use crate::error::DxfError;
        use crate::io::dxf::GroupCodeValueType;

        let too_big = || {
            DxfError::Parse(format!(
                "value {value} does not fit the width binary DXF gives group {code}"
            ))
        };
        let width = match GroupCodeValueType::from_raw_code(code) {
            GroupCodeValueType::Bool => IntWidth::One,
            GroupCodeValueType::Byte | GroupCodeValueType::Int16 => IntWidth::Two,
            GroupCodeValueType::Int32 => IntWidth::Four,
            GroupCodeValueType::Int64 => IntWidth::Eight,
            GroupCodeValueType::Double | GroupCodeValueType::Point3D => {
                self.write_code(code)?;
                self.writer.write_f64::<LittleEndian>(value as f64)?;
                return Ok(());
            }
            _ => native,
        };
        match width {
            IntWidth::One => {
                let v = u8::try_from(value).ok().filter(|v| *v <= 1).ok_or_else(too_big)?;
                self.write_code(code)?;
                self.writer.write_u8(v)?;
            }
            IntWidth::Two => {
                let v = i16::try_from(value).map_err(|_| too_big())?;
                self.write_code(code)?;
                self.writer.write_i16::<LittleEndian>(v)?;
            }
            IntWidth::Four => {
                let v = i32::try_from(value).map_err(|_| too_big())?;
                self.write_code(code)?;
                self.writer.write_i32::<LittleEndian>(v)?;
            }
            IntWidth::Eight => {
                self.write_code(code)?;
                self.writer.write_i64::<LittleEndian>(value)?;
            }
        }
        Ok(())
    }

    /// Get the inner writer
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<W: Write> DxfStreamWriter for DxfBinaryWriter<W> {
    fn is_binary(&self) -> bool {
        true
    }

    fn write_string(&mut self, code: i32, value: &str) -> Result<()> {
        if self.write_typed_text(code, value)? {
            return Ok(());
        }
        self.write_code(code)?;
        // Sanitize embedded newlines to DXF paragraph markers, matching the
        // ASCII writer.  While binary DXF uses null-terminated strings (so raw
        // newlines won't structurally break the file), many AutoCAD consumers
        // still expect \P instead of literal line breaks in string values.
        if value.contains('\n') || value.contains('\r') {
            let sanitized = value
                .replace("\r\n", "\\P")
                .replace('\r', "\\P")
                .replace('\n', "\\P");
            self.write_null_string(&sanitized)?;
        } else {
            self.write_null_string(value)?;
        }
        Ok(())
    }

    fn write_xrecord_string(&mut self, code: i32, value: &str) -> Result<()> {
        if self.write_typed_text(code, value)? {
            return Ok(());
        }
        self.write_code(code)?;
        self.write_null_string(value)
    }

    fn write_byte(&mut self, code: i32, value: u8) -> Result<()> {
        // Group codes 280-289 are "Byte" type but written as Int16 in binary DXF
        self.write_integer(code, i64::from(value), IntWidth::Two)
    }

    fn write_i16(&mut self, code: i32, value: i16) -> Result<()> {
        self.write_integer(code, i64::from(value), IntWidth::Two)
    }

    fn write_i32(&mut self, code: i32, value: i32) -> Result<()> {
        self.write_integer(code, i64::from(value), IntWidth::Four)
    }

    fn write_i64(&mut self, code: i32, value: i64) -> Result<()> {
        self.write_integer(code, value, IntWidth::Eight)
    }

    fn write_double(&mut self, code: i32, value: f64) -> Result<()> {
        self.write_code(code)?;
        self.writer.write_f64::<LittleEndian>(value)?;
        Ok(())
    }

    fn write_bool(&mut self, code: i32, value: bool) -> Result<()> {
        // Logical flags also occur in integer groups, including history
        // flags 280/281. Their binary width follows the group code, not the
        // Rust value type; only groups 290-299 use a single byte.
        self.write_integer(code, i64::from(value), IntWidth::One)
    }

    fn write_handle(&mut self, code: i32, handle: Handle) -> Result<()> {
        self.write_code(code)?;
        // Handles are written as hex strings even in binary DXF
        // Format directly into stack buffer to avoid heap allocation
        let val = handle.value();
        if val == 0 {
            self.writer.write_all(b"0\0")?;
        } else {
            let mut pos = 16usize;
            let mut v = val;
            while v > 0 {
                pos -= 1;
                let digit = (v & 0xF) as u8;
                self.hex_buf[pos] = if digit < 10 {
                    b'0' + digit
                } else {
                    b'A' + digit - 10
                };
                v >>= 4;
            }
            let hex_len = 16 - pos;
            self.hex_buf.copy_within(pos..16, 0);
            self.hex_buf[hex_len] = 0; // null terminator
            self.writer.write_all(&self.hex_buf[..hex_len + 1])?;
        }
        Ok(())
    }

    fn write_binary(&mut self, code: i32, data: &[u8]) -> Result<()> {
        // Each binary group has a one-byte length. Repeat the group instead
        // of truncating the length when a caller supplies a larger payload.
        if data.is_empty() {
            self.write_code(code)?;
            self.writer.write_u8(0)?;
        }
        for chunk in data.chunks(255) {
            self.write_code(code)?;
            self.writer.write_u8(chunk.len() as u8)?;
            self.writer.write_all(chunk)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_sentinel() {
        let mut buf = Vec::new();
        {
            let _writer = DxfBinaryWriter::new(&mut buf).unwrap();
        }
        assert!(buf.starts_with(BINARY_DXF_SENTINEL));
    }

    #[test]
    fn test_write_string() {
        let mut buf = Vec::new();
        {
            let mut writer = DxfBinaryWriter::new(&mut buf).unwrap();
            writer.write_string(0, "LINE").unwrap();
        }
        let sentinel_len = BINARY_DXF_SENTINEL.len();
        // After sentinel: code (2 bytes) + string + null
        assert_eq!(buf[sentinel_len..sentinel_len + 2], [0, 0]); // code 0 as little-endian
        assert_eq!(&buf[sentinel_len + 2..sentinel_len + 6], b"LINE");
        assert_eq!(buf[sentinel_len + 6], 0); // null terminator
    }

    #[test]
    fn test_write_double() {
        let mut buf = Vec::new();
        {
            let mut writer = DxfBinaryWriter::new(&mut buf).unwrap();
            writer.write_double(10, 1.5).unwrap();
        }
        let sentinel_len = BINARY_DXF_SENTINEL.len();
        // code (2 bytes) + f64 (8 bytes)
        assert_eq!(buf[sentinel_len..sentinel_len + 2], [10, 0]); // code 10 as little-endian
                                                                  // 1.5 as f64 little-endian
        let expected: [u8; 8] = 1.5f64.to_le_bytes();
        assert_eq!(&buf[sentinel_len + 2..sentinel_len + 10], &expected);
    }

    #[test]
    fn test_write_i16() {
        let mut buf = Vec::new();
        {
            let mut writer = DxfBinaryWriter::new(&mut buf).unwrap();
            writer.write_i16(62, 7).unwrap();
        }
        let sentinel_len = BINARY_DXF_SENTINEL.len();
        assert_eq!(buf[sentinel_len..sentinel_len + 2], [62, 0]); // code 62
        assert_eq!(buf[sentinel_len + 2..sentinel_len + 4], [7, 0]); // value 7
    }

    #[test]
    fn test_write_string_newline_sanitization() {
        let mut buf = Vec::new();
        {
            let mut writer = DxfBinaryWriter::new(&mut buf).unwrap();
            writer.write_string(1, "Hello\r\nWorld\nFoo\rBar").unwrap();
        }
        let sentinel_len = BINARY_DXF_SENTINEL.len();
        // code (2 bytes) + sanitized string + null
        let str_start = sentinel_len + 2;
        let str_end = buf[str_start..].iter().position(|&b| b == 0).unwrap() + str_start;
        let written = std::str::from_utf8(&buf[str_start..str_end]).unwrap();
        assert_eq!(written, "Hello\\PWorld\\PFoo\\PBar");
    }
}

/// The width of a group's value in binary DXF is fixed by its code, whatever
/// Rust type the caller holds. These tests hold the writer to the table in
/// the DXF reference, code by code (acadsharp-rs-tests#110).
#[cfg(test)]
mod width_tests {
    use super::*;

    /// Bytes a value of this code occupies after the two-byte code, or None
    /// for the codes whose value is a string or a length-prefixed chunk.
    fn width(code: i32) -> Option<usize> {
        match code {
            10..=59 | 110..=149 | 210..=239 | 460..=469 | 1010..=1059 => Some(8),
            60..=79 | 170..=179 | 270..=289 | 370..=389 | 400..=409 | 1060..=1070 => Some(2),
            90..=99 | 420..=429 | 440..=459 | 1071 => Some(4),
            160..=169 => Some(8),
            290..=299 => Some(1),
            _ => None,
        }
    }

    fn written(f: impl FnOnce(&mut DxfBinaryWriter<&mut Vec<u8>>) -> Result<()>) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = DxfBinaryWriter::new(&mut buf).unwrap();
            f(&mut w).unwrap();
        }
        buf[BINARY_DXF_SENTINEL.len()..].to_vec()
    }

    #[test]
    fn integers_take_the_width_of_the_code_and_not_of_the_rust_type() {
        for code in 0..=1071 {
            let Some(w) = width(code) else { continue };
            if w == 8 && !(160..=169).contains(&code) {
                continue; // doubles: covered below
            }
            for (name, bytes) in [
                ("write_i16", written(|x| x.write_i16(code, 1))),
                ("write_i32", written(|x| x.write_i32(code, 1))),
                ("write_i64", written(|x| x.write_i64(code, 1))),
                ("write_byte", written(|x| x.write_byte(code, 1))),
                ("write_bool", written(|x| x.write_bool(code, true))),
            ] {
                assert_eq!(
                    bytes.len(),
                    2 + w,
                    "{name} on group {code} wrote {} value bytes, the format has {w}",
                    bytes.len().saturating_sub(2)
                );
                assert_eq!(bytes[0..2], (code as i16).to_le_bytes());
                assert_eq!(bytes[2], 1, "{name} on group {code} lost the value");
            }
        }
    }

    #[test]
    fn a_double_takes_eight_bytes_on_a_double_code() {
        for code in (10..=59).chain(110..=149).chain(210..=239).chain(460..=469) {
            let bytes = written(|x| x.write_double(code, 1.5));
            assert_eq!(bytes.len(), 10, "group {code}");
            assert_eq!(bytes[2..], 1.5f64.to_le_bytes());
        }
    }

    #[test]
    fn a_value_that_does_not_fit_its_group_is_an_error_and_not_a_truncation() {
        let mut buf = Vec::new();
        let mut w = DxfBinaryWriter::new(&mut buf).unwrap();
        assert!(w.write_i32(73, 70_000).is_err());
        assert!(w.write_i32(290, 2).is_err());
    }
}
