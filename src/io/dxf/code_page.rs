//! DXF code page ($DWGCODEPAGE) to encoding mapping.
//!
//! Maps DXF code page names to `encoding_rs` encodings, following the same
//! mapping table used by the reference `CadUtils._dxfEncodingMap`.

use encoding_rs::Encoding;

/// Get the `encoding_rs` encoding for a DXF code page string.
///
/// Returns `None` if the encoding is UTF-8 (no transcoding needed) or the
/// code page string is not recognized.
///
/// # Rules
/// - If the DXF version is AC1021 (AutoCAD 2007+) or later, UTF-8 is always
///   used regardless of $DWGCODEPAGE — callers should not call this function.
/// - Otherwise, the code page string (case-insensitive) is looked up in the
///   mapping table.
pub fn encoding_from_code_page(code_page: &str) -> Option<&'static Encoding> {
    match code_page.to_ascii_lowercase().as_str() {
        // Asian encodings
        "gb2312" | "ansi_936" => Some(encoding_rs::GBK),
        "big5" | "ansi_950" => Some(encoding_rs::BIG5),
        "korean" | "ansi_949" | "johab" => Some(encoding_rs::EUC_KR),
        "ansi_932" => Some(encoding_rs::SHIFT_JIS),

        // DOS/OEM code pages
        "dos437" => Some(encoding_rs::IBM866), // closest available in encoding_rs
        "dos850" => Some(encoding_rs::WINDOWS_1252), // Western European
        "dos852" => Some(encoding_rs::WINDOWS_1250), // Central European
        "dos855" | "dos866" => Some(encoding_rs::IBM866), // Cyrillic
        "dos857" => Some(encoding_rs::WINDOWS_1254), // Turkish
        "dos860" => Some(encoding_rs::WINDOWS_1252), // Portuguese
        "dos861" => Some(encoding_rs::WINDOWS_1252), // Icelandic
        "dos863" => Some(encoding_rs::WINDOWS_1252), // Canadian-French
        "dos865" => Some(encoding_rs::WINDOWS_1252), // Nordic
        "dos869" => Some(encoding_rs::WINDOWS_1253), // Greek

        // Windows/ANSI code pages
        "ansi_874" => Some(encoding_rs::WINDOWS_874),
        "ansi_1250" => Some(encoding_rs::WINDOWS_1250),
        "ansi_1251" => Some(encoding_rs::WINDOWS_1251),
        "ansi_1252" => Some(encoding_rs::WINDOWS_1252),
        "ansi_1253" => Some(encoding_rs::WINDOWS_1253),
        "ansi_1254" => Some(encoding_rs::WINDOWS_1254),
        "ansi_1255" => Some(encoding_rs::WINDOWS_1255),
        "ansi_1256" => Some(encoding_rs::WINDOWS_1256),
        "ansi_1257" => Some(encoding_rs::WINDOWS_1257),
        "ansi_1258" => Some(encoding_rs::WINDOWS_1258),

        // ISO encodings
        "iso8859-1" | "iso_8859-1" => Some(encoding_rs::WINDOWS_1252),
        "iso8859-2" | "iso_8859-2" => Some(encoding_rs::ISO_8859_2),
        "iso8859-3" | "iso_8859-3" => Some(encoding_rs::ISO_8859_3),
        "iso8859-4" | "iso_8859-4" => Some(encoding_rs::ISO_8859_4),
        "iso8859-5" | "iso_8859-5" => Some(encoding_rs::ISO_8859_5),
        "iso8859-6" | "iso_8859-6" => Some(encoding_rs::ISO_8859_6),
        "iso8859-7" | "iso_8859-7" => Some(encoding_rs::ISO_8859_7),
        "iso8859-8" | "iso_8859-8" => Some(encoding_rs::ISO_8859_8),
        "iso8859-9" | "iso_8859-9" => Some(encoding_rs::WINDOWS_1254),
        "iso8859-10" | "iso_8859-10" => Some(encoding_rs::ISO_8859_10),
        "iso8859-13" | "iso_8859-13" => Some(encoding_rs::ISO_8859_13),
        "iso8859-14" | "iso_8859-14" => Some(encoding_rs::ISO_8859_14),
        "iso8859-15" | "iso_8859-15" => Some(encoding_rs::ISO_8859_15),

        // KOI8-R (Russian)
        "koi8-r" => Some(encoding_rs::KOI8_R),
        "koi8-u" => Some(encoding_rs::KOI8_U),

        // ASCII / UTF-8 / no fallback needed
        "ascii" | "utf-8" | "utf8" | "unicode" => None,

        // Default: Windows-1252 (most common DXF fallback)
        _ => Some(encoding_rs::WINDOWS_1252),
    }
}

/// Resolve the compact code-page index stored in DWG file metadata.
pub fn dwg_code_page_name(index: u16) -> &'static str {
    match index {
        1 => "ASCII",
        2 => "ISO8859-1",
        3 => "ISO8859-2",
        4 => "ISO8859-3",
        5 => "ISO8859-4",
        6 => "ISO8859-5",
        7 => "ISO8859-6",
        8 => "ISO8859-7",
        9 => "ISO8859-8",
        10 => "ISO8859-9",
        11 => "DOS437",
        12 => "DOS850",
        13 => "DOS852",
        14 => "DOS855",
        15 => "DOS857",
        16 => "DOS860",
        17 => "DOS861",
        18 => "DOS863",
        19 => "DOS864",
        20 => "DOS865",
        21 => "DOS869",
        22 | 38 => "ANSI_932",
        23 => "MAC-ROMAN",
        24 | 41 => "BIG5",
        25 | 40 => "KOREAN",
        26 | 42 => "JOHAB",
        27 => "DOS866",
        28 => "ANSI_1250",
        29 => "ANSI_1251",
        30 => "ANSI_1252",
        31 | 39 => "GB2312",
        32 => "ANSI_1253",
        33 => "ANSI_1254",
        34 => "ANSI_1255",
        35 => "ANSI_1256",
        36 => "ANSI_1257",
        37 => "ANSI_874",
        43 => "UTF-8",
        44 => "ANSI_1258",
        _ => "ANSI_1252",
    }
}

/// Convert a DXF `$DWGCODEPAGE` name to the compact DWG metadata index.
pub fn dwg_code_page_index(code_page: &str) -> u16 {
    match code_page.to_ascii_lowercase().as_str() {
        "ascii" => 1,
        "iso8859-1" | "iso_8859-1" => 2,
        "iso8859-2" | "iso_8859-2" => 3,
        "iso8859-3" | "iso_8859-3" => 4,
        "iso8859-4" | "iso_8859-4" => 5,
        "iso8859-5" | "iso_8859-5" => 6,
        "iso8859-6" | "iso_8859-6" => 7,
        "iso8859-7" | "iso_8859-7" => 8,
        "iso8859-8" | "iso_8859-8" => 9,
        "iso8859-9" | "iso_8859-9" => 10,
        "dos437" => 11,
        "dos850" => 12,
        "dos852" => 13,
        "dos855" => 14,
        "dos857" => 15,
        "dos860" => 16,
        "dos861" => 17,
        "dos863" => 18,
        "dos864" => 19,
        "dos865" => 20,
        "dos869" => 21,
        "ansi_932" | "dos932" => 22,
        "mac-roman" => 23,
        "big5" | "ansi_950" | "dos950" => 24,
        "korean" | "ansi_949" => 25,
        "johab" => 26,
        "dos866" => 27,
        "ansi_1250" | "ansi1250" => 28,
        "ansi_1251" | "ansi1251" => 29,
        "ansi_1252" | "ansi1252" => 30,
        "gb2312" | "ansi_936" => 31,
        "ansi_1253" | "ansi1253" => 32,
        "ansi_1254" | "ansi1254" => 33,
        "ansi_1255" | "ansi1255" => 34,
        "ansi_1256" | "ansi1256" => 35,
        "ansi_1257" | "ansi1257" => 36,
        "ansi_874" => 37,
        "utf-8" | "utf8" | "unicode" => 43,
        "ansi_1258" | "ansi1258" => 44,
        _ => 30,
    }
}

/// A drawing's declared legacy code page: the `encoding_rs` row its bytes are
/// transcoded with, paired with the only other thing the MIF gate has to know
/// — whether that row's repertoire *is* the code page's, or merely the
/// "closest available in `encoding_rs`" the table above settles for.
///
/// The pairing cannot be flattened back to the encoding pointer. The table in
/// [`encoding_from_code_page`] is many-to-one, and on four of its rows the two
/// sides of one pointer need opposite answers:
///
/// | `encoding_rs` row | faithful for | stand-in for |
/// | --- | --- | --- |
/// | `windows-1252` | `ANSI_1252` | `MAC-ROMAN`, `DOS850`, `DOS860`, `DOS861`, `DOS863`, `DOS865`, `ISO8859-1`, any unrecognised name |
/// | `windows-1250` | `ANSI_1250` | `DOS852` |
/// | `windows-1253` | `ANSI_1253` | `DOS869` |
/// | `windows-1254` | `ANSI_1254` | `DOS857`, `ISO8859-9` |
///
/// `MAC-ROMAN` and `ANSI_1252` are the pair that proves it, and
/// `mac_roman_and_ansi_1252_alias_one_encoding` pins it: both resolve to
/// windows-1252, yet a `\U+00DE` escape has to decode in a `MAC-ROMAN`
/// drawing (Mac OS Roman has no `Þ`, so only a writer's escape could have put
/// it there) and stay literal in an `ANSI_1252` one (windows-1252 holds `Þ` at
/// `0xDE`, so nothing escaped it). A function handed only `&'static Encoding`
/// cannot be right about both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyCodePage {
    encoding: &'static Encoding,
    faithful: bool,
}

impl LegacyCodePage {
    /// The `encoding_rs` row to transcode this code page's bytes with.
    pub fn encoding(self) -> &'static Encoding {
        self.encoding
    }

    /// Whether [`Self::encoding`] may be asked which characters the drawing
    /// could not have held — true only where the row is the code page itself.
    pub fn is_faithful(self) -> bool {
        self.faithful
    }

    /// Transcode `bytes` from this code page, forwarding to
    /// [`Encoding::decode`].
    ///
    /// This is an inherent method and not a `Deref` to `Encoding` because
    /// `Encoding::decode` takes `&'static self`, and a `Deref` borrow of a
    /// local cannot supply that lifetime; the `&'static Encoding` this value
    /// carries can.
    pub fn decode<'a>(
        self,
        bytes: &'a [u8],
    ) -> (std::borrow::Cow<'a, str>, &'static Encoding, bool) {
        self.encoding.decode(bytes)
    }
}

/// A bare encoding names no code page, so the gate has nothing to stand on and
/// every escape decodes — exactly the behaviour callers had before the gate
/// existed. Paths that do know the drawing's code page carry a
/// [`LegacyCodePage`] instead and get the gate.
impl From<&'static Encoding> for LegacyCodePage {
    fn from(encoding: &'static Encoding) -> Self {
        Self {
            encoding,
            faithful: false,
        }
    }
}

/// Whether the row [`encoding_from_code_page`] picks for `code_page` has that
/// code page's repertoire.
///
/// Only a faithful row can answer "could this drawing have held this
/// character?". The rest are approximations, and asking them yields an answer
/// about a different character set: CP437 has no Cyrillic at all where its
/// stand-in IBM866 is almost nothing but Cyrillic; CP932 maps neither `U+00A5`
/// nor `U+203E`, where WHATWG Shift_JIS does; the WHATWG `Big5` row is a
/// superset of the code page it stands in for (`Big5` is HKSCS, CP950 is
/// not), while the `EUC-KR` row *is* CP949 — which is why `ANSI_949` and
/// `KOREAN` are candidates for the faithful list rather than permanent
/// stand-ins (`#9`);
/// `MAC-ROMAN` has no arm at all and lands on the windows-1252 default. An
/// unrecognised name reaches that same default and is a stand-in for whatever
/// the drawing really declared.
///
/// Note that a spelling the table does not match is not faithful even when a
/// matched spelling of the same code page would be: `ansi1252` misses the
/// `"ansi_1252"` arm and lands on the default, so the encoder it gets was not
/// chosen for it.
pub fn code_page_is_faithful(code_page: &str) -> bool {
    matches!(
        code_page.to_ascii_lowercase().as_str(),
        // Windows/ANSI rows: WHATWG windows-125x is CP125x except in the
        // unassigned C1 slots U+0081..U+009F, which WHATWG maps to C1
        // controls and Microsoft leaves undefined; no glyph, no drawing
        // content. ANSI_1255 is the one real exception and is not on this
        // list: WHATWG maps U+05BA at 0xCA where CP1255 leaves it undefined.
        "ansi_1250"
            | "ansi_1251"
            | "ansi_1252"
            | "ansi_1253"
            | "ansi_1254"
            | "ansi_1256"
            | "ansi_1257"
            | "ansi_1258"
            | "ansi_874"
            // ISO rows that map to their own `encoding_rs` row. ISO8859-1 and
            // ISO8859-9 are absent on purpose: they borrow windows-1252 and
            // windows-1254.
            | "iso8859-2"
            | "iso_8859-2"
            | "iso8859-3"
            | "iso_8859-3"
            | "iso8859-4"
            | "iso_8859-4"
            | "iso8859-5"
            | "iso_8859-5"
            | "iso8859-6"
            | "iso_8859-6"
            | "iso8859-7"
            | "iso_8859-7"
            | "iso8859-8"
            | "iso_8859-8"
            // KOI8-R and ISO8859-10/13/14/15 are faithful rows that a DWG
            // write cannot reach: `dwg_code_page_index` has no arm for them
            // (nor for KOI8-U, which left this list above) and falls to 30,
            // so the file declares ANSI_1252 while the bytes are in this row.
            // Tracked in `hakanaktt/acadrust#102`.
            | "iso8859-10"
            | "iso_8859-10"
            | "iso8859-13"
            | "iso_8859-13"
            | "iso8859-14"
            | "iso_8859-14"
            | "iso8859-15"
            | "iso_8859-15"
            | "koi8-r"
            // GBK is CP936's row here, and the two agree on 23907 of GBK's
            // 24085 BMP scalars. The 178 GBK-only ones — U+1E3F,
            // U+9FB4..U+9FBB, U+FE10..U+FE19 and 159 PUA — are confirmed
            // absent from CP936 by unicode.org's CP936.TXT and by
            // per-character iconv, so an escape naming one of those is
            // transport this gate calls content. Tracked in `#9`.
            | "gb2312"
            | "ansi_936"
    )
}

/// Resolve a `$DWGCODEPAGE` name to the code page to read and write its
/// strings with.
///
/// `None` where [`encoding_from_code_page`] needs no transcoding (ASCII,
/// UTF-8).
pub fn legacy_code_page(code_page: &str) -> Option<LegacyCodePage> {
    Some(LegacyCodePage {
        encoding: encoding_from_code_page(code_page)?,
        faithful: code_page_is_faithful(code_page),
    })
}

/// Resolve the compact DWG metadata index to the code page its strings are
/// stored in.
///
/// Callers that only transcode bytes take [`LegacyCodePage::encoding`] or
/// [`LegacyCodePage::decode`]; callers that decode MIF escapes also get the
/// answer to whether that row is trustworthy for the declared code page via
/// [`LegacyCodePage::is_faithful`]. There is deliberately no `Deref` — see
/// [`LegacyCodePage::decode`] for why.
pub fn encoding_from_dwg_code_page(index: u16) -> LegacyCodePage {
    let name = dwg_code_page_name(index);
    LegacyCodePage {
        encoding: encoding_from_code_page(name).unwrap_or(encoding_rs::WINDOWS_1252),
        // An index outside the table lands on the `ANSI_1252` default name,
        // which is faithful for the code page that really is ANSI_1252 and a
        // stand-in for whatever this drawing actually declared.
        faithful: (1..=44).contains(&index) && code_page_is_faithful(name),
    }
}

/// Encode a string to a legacy (pre-UTF-16) DWG code page.
///
/// Characters the code page cannot represent are emitted as AutoCAD MIF
/// `\U+XXXX` escapes (astral-plane characters as a surrogate pair of
/// escapes) instead of `encoding_rs`'s HTML `&#NNNNN;` references, which
/// no CAD application understands. Well-formed MIF escapes round-trip
/// through [`decode_mif_escapes`].
pub fn encode_legacy_string(text: &str, encoding: &'static Encoding) -> Vec<u8> {
    let (encoded, _, unmappable) = encoding.encode(text);
    if !unmappable {
        return encoded.into_owned();
    }
    let mut out = Vec::with_capacity(text.len() + 6);
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        // Every DWG code page is stateless, so per-char encoding is safe.
        let (bytes, _, err) = encoding.encode(ch.encode_utf8(&mut buf));
        if err {
            let mut units = [0u16; 2];
            for unit in ch.encode_utf16(&mut units) {
                out.extend_from_slice(format!("\\U+{:04X}", unit).as_bytes());
            }
        } else {
            out.extend_from_slice(&bytes);
        }
    }
    out
}

/// Decode AutoCAD MIF `\U+XXXX` escapes (exactly four hex digits) into
/// Unicode characters, but only where the escape is transport and not
/// content.
///
/// MIF exists solely to carry the characters a code page cannot hold:
/// [`encode_legacy_string`] escapes a character only when
/// `encoding.encode(ch)` reports an unmappable character, and emits it
/// verbatim otherwise. So an escape naming a character the code page *can*
/// represent was never produced by that mechanism — it is literal drawing
/// content, and the characters the file holds go back out as they are. The
/// test is the encoder's own condition, applied to the combined scalar
/// rather than to each UTF-16 unit, because [`encode_legacy_string`]
/// escapes an astral character as a surrogate pair.
///
/// The asymmetry was observable: `real_AC1018` carries the MTEXT content
/// `94\U+00B0` in an ANSI_1252 drawing, where `U+00B0` is representable and
/// nothing escaped it, yet an ungated decode turned those eight characters
/// into `94°` on the way back in. `real_AC1032` is R2007+, stores text as
/// UTF-16 and never reaches this path, so it shows the same content intact.
///
/// That condition may only be put to a code page whose `encoding_rs` row is
/// the code page — [`LegacyCodePage::is_faithful`]. Where the row is a
/// stand-in, or where the caller passes a bare `&'static Encoding` and so
/// names no code page at all, every well-formed escape decodes, which is
/// what this function did before the gate existed. Putting the question to a
/// stand-in answers about the wrong character set: IBM866, asked what a
/// `DOS437` drawing could hold, called a `\U+0411` transport escape content
/// and left seven literal characters in the text of a Cyrillic drawing.
///
/// A high-surrogate escape followed by a low-surrogate escape combines into
/// a scalar value. Malformed, unterminated and lone-surrogate escapes are
/// left as literal text. A lone surrogate names no scalar, so no encoder
/// can have escaped one: such an escape is content by the same test as any
/// other, seven representable ASCII characters that have to cross verbatim.
pub fn decode_mif_escapes(text: &str, code_page: impl Into<LegacyCodePage>) -> String {
    decode_mif_escapes_inner(text, code_page.into())
}

fn decode_mif_escapes_inner(text: &str, code_page: LegacyCodePage) -> String {
    if !text.contains("\\U+") {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let hex_at = |start: usize| -> Option<u16> {
        if start + 4 > len {
            return None;
        }
        let hex: String = chars[start..start + 4].iter().collect();
        u16::from_str_radix(&hex, 16).ok()
    };
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < len {
        if chars[i] == '\\' && i + 7 <= len && chars[i + 1] == 'U' && chars[i + 2] == '+' {
            if let Some(unit) = hex_at(i + 3) {
                // Where the escape turns out to be content, the characters
                // the file actually holds are what goes back out, copied
                // from here rather than re-spelled: `hex_at` accepts
                // lower-case digits, so re-formatting one would rewrite
                // `\U+00b0` as `\U+00B0` in the very branch whose job is to
                // leave content alone.
                let escape_start = i;
                i += 7;
                let mut units = [unit, 0];
                let mut count = 1usize;
                // Combine a following low-surrogate escape into one scalar.
                if (0xD800..0xDC00).contains(&unit)
                    && i + 7 <= len
                    && chars[i] == '\\'
                    && chars[i + 1] == 'U'
                    && chars[i + 2] == '+'
                {
                    if let Some(low) = hex_at(i + 3) {
                        if (0xDC00..0xE000).contains(&low) {
                            units[1] = low;
                            count = 2;
                            i += 7;
                        }
                    }
                }
                let scalar = std::char::decode_utf16(units[..count].iter().copied()).next();
                match scalar {
                    // A faithful code page that can hold `ch` says nothing
                    // escaped it, so the escape is content and the
                    // characters the file holds go back out. `is_faithful`
                    // is checked first: a stand-in row is never asked, and
                    // never pays for the per-character encode.
                    Some(Ok(ch)) if code_page.is_faithful() && holds(code_page.encoding(), ch) => {
                        out.extend(chars[escape_start..i].iter().copied());
                    }
                    Some(Ok(ch)) => out.push(ch),
                    // A lone surrogate: not a scalar, so unproducible by
                    // `encode_legacy_string`, which emits an astral
                    // character as a *pair* of escapes. It can only be
                    // literal file content, and the characters the file
                    // holds go back out.
                    _ => out.extend(chars[escape_start..i].iter().copied()),
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Whether `encoding` can represent `ch`, which is the exact condition
/// [`encode_legacy_string`] escapes on. Every DWG code page is stateless, so
/// asking one character at a time is safe.
fn holds(encoding: &'static Encoding, ch: char) -> bool {
    let mut buf = [0u8; 4];
    let (_, _, unmappable) = encoding.encode(ch.encode_utf8(&mut buf));
    !unmappable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_1252() {
        let enc = encoding_from_code_page("ANSI_1252");
        assert_eq!(enc, Some(encoding_rs::WINDOWS_1252));
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(
            encoding_from_code_page("ansi_1251"),
            encoding_from_code_page("ANSI_1251")
        );
    }

    #[test]
    fn test_ascii_returns_none() {
        assert_eq!(encoding_from_code_page("ASCII"), None);
    }

    #[test]
    fn test_utf8_returns_none() {
        assert_eq!(encoding_from_code_page("UTF-8"), None);
    }

    #[test]
    fn test_unknown_returns_windows1252() {
        let enc = encoding_from_code_page("SOMETHING_UNKNOWN");
        assert_eq!(enc, Some(encoding_rs::WINDOWS_1252));
    }

    #[test]
    fn test_asian_encodings() {
        assert_eq!(encoding_from_code_page("GB2312"), Some(encoding_rs::GBK));
        assert_eq!(encoding_from_code_page("BIG5"), Some(encoding_rs::BIG5));
        assert_eq!(
            encoding_from_code_page("ANSI_932"),
            Some(encoding_rs::SHIFT_JIS)
        );
        assert_eq!(encoding_from_code_page("KOREAN"), Some(encoding_rs::EUC_KR));
    }

    #[test]
    fn test_decode_mif_escapes() {
        let ansi_1252 = legacy_code_page("ANSI_1252").unwrap();
        assert_eq!(decode_mif_escapes("ab\\U+4E2Dcd", ansi_1252), "ab中cd");
        // The gate is the encoder's own condition: under a code page whose
        // encoder is that code page, an escape naming a character it can
        // represent was never written by MIF, so it is content and stays
        // literal. Only unrepresentable escapes are transport and decode.
        assert_eq!(
            decode_mif_escapes("\\U+0041\\U+0042", ansi_1252),
            "\\U+0041\\U+0042"
        );
        assert_eq!(decode_mif_escapes("\\U+2205", ansi_1252), "∅");
        // The same escape flips with the code page: Cyrillic 1251 has no ø.
        assert_eq!(decode_mif_escapes("\\U+00F8", ansi_1252), "\\U+00F8");
        assert_eq!(
            decode_mif_escapes("\\U+00F8", legacy_code_page("ANSI_1251").unwrap()),
            "ø"
        );
        // Exactly four hex digits: a fifth character stays literal.
        assert_eq!(decode_mif_escapes("\\U+22052", ansi_1252), "∅2");
        // Malformed escapes stay literal.
        assert_eq!(decode_mif_escapes("\\U+GGGG", ansi_1252), "\\U+GGGG");
        assert_eq!(decode_mif_escapes("\\U+4E", ansi_1252), "\\U+4E");
        assert_eq!(
            decode_mif_escapes("no escapes here", ansi_1252),
            "no escapes here"
        );
        // A lone surrogate names no scalar, so no writer escaped one: the
        // seven characters the file holds cross verbatim instead of being
        // deleted.
        assert_eq!(decode_mif_escapes("\\U+D800", ansi_1252), "\\U+D800");
        assert_eq!(decode_mif_escapes("A\\U+D800B", ansi_1252), "A\\U+D800B");
        assert_eq!(
            decode_mif_escapes("\\U+DC00\\U+D800", ansi_1252),
            "\\U+DC00\\U+D800"
        );
        // Surrogate pair combines into a scalar.
        assert_eq!(decode_mif_escapes("\\U+D83D\\U+DE00", ansi_1252), "😀");
    }

    #[test]
    fn mac_roman_and_ansi_1252_alias_one_encoding() {
        // Why `LegacyCodePage` carries the faithfulness answer instead of
        // deriving it from the encoding: the table is many-to-one, and each
        // pair below is one `encoding_rs` row serving a code page it *is* and
        // a code page it only stands in for.
        for (stand_in, faithful) in [
            (23u16, 30u16), // MAC-ROMAN, ANSI_1252
            (2, 30),        // ISO8859-1, ANSI_1252
            (13, 28),       // DOS852,    ANSI_1250
            (21, 32),       // DOS869,    ANSI_1253
            (15, 33),       // DOS857,    ANSI_1254
            (10, 33),       // ISO8859-9, ANSI_1254
        ] {
            let stand_in_page = encoding_from_dwg_code_page(stand_in);
            let faithful_page = encoding_from_dwg_code_page(faithful);
            assert_eq!(
                stand_in_page.encoding(),
                faithful_page.encoding(),
                "{} and {} no longer share a row",
                dwg_code_page_name(stand_in),
                dwg_code_page_name(faithful)
            );
            assert!(
                !stand_in_page.is_faithful(),
                "{} is a stand-in",
                dwg_code_page_name(stand_in)
            );
            assert!(
                faithful_page.is_faithful(),
                "{} is its own encoder",
                dwg_code_page_name(faithful)
            );
        }
        // And the same escape needs opposite answers on one of those rows.
        assert_eq!(dwg_code_page_name(23), "MAC-ROMAN");
        assert_eq!(
            decode_mif_escapes("\\U+00DE", encoding_from_dwg_code_page(23)),
            "Þ"
        );
        assert_eq!(
            decode_mif_escapes("\\U+00DE", encoding_from_dwg_code_page(30)),
            "\\U+00DE"
        );
    }

    #[test]
    fn stand_in_code_pages_decode_every_escape() {
        // The rows the gate regressed while it trusted the table. IBM866
        // stands in for CP437, which holds no Cyrillic at all, so a `\U+0411`
        // in a DOS437 drawing can only be transport — yet IBM866 answers that
        // it is content. WHATWG Shift_JIS maps U+00A5 and U+203E where CP932
        // does not. MAC-ROMAN has no arm and lands on the windows-1252
        // default.
        let dos437 = legacy_code_page("DOS437").unwrap();
        assert_eq!(dos437.encoding(), encoding_rs::IBM866);
        assert!(
            holds(encoding_rs::IBM866, 'Б'),
            "the stand-in claims CP437 can hold Cyrillic; that is the bug"
        );
        assert_eq!(decode_mif_escapes("\\U+0411", dos437), "Б");

        let ansi_932 = legacy_code_page("ANSI_932").unwrap();
        assert_eq!(ansi_932.encoding(), encoding_rs::SHIFT_JIS);
        assert_eq!(decode_mif_escapes("\\U+00A5", ansi_932), "¥");
        assert_eq!(decode_mif_escapes("\\U+203E", ansi_932), "‾");

        let mac_roman = legacy_code_page("MAC-ROMAN").unwrap();
        assert_eq!(decode_mif_escapes("\\U+00DE", mac_roman), "Þ");

        // WHATWG windows-1255 maps U+05BA (HEBREW POINT HOLAM HASER FOR VAV)
        // at 0xCA; Microsoft's CP1255 leaves 0xCA undefined, so a real CP1255
        // writer had to escape it and the escape is transport.
        let ansi_1255 = legacy_code_page("ANSI_1255").unwrap();
        assert_eq!(decode_mif_escapes("\\U+05BA", ansi_1255), "\u{05BA}");
    }

    #[test]
    fn faithful_code_pages_keep_a_representable_escape_literal() {
        // The two rows AutoCAD itself certifies, via `real_AC1018` (R2004,
        // MIF on disk) against `real_AC1032` (R2018, UTF-16, same content).
        let ansi_1252 = legacy_code_page("ANSI_1252").unwrap();
        assert!(ansi_1252.is_faithful());
        assert_eq!(decode_mif_escapes("94\\U+00B0", ansi_1252), "94\\U+00B0");
        assert_eq!(decode_mif_escapes("\\U+220545,6", ansi_1252), "∅45,6");
        // GBK is CP936, so the gate holds for Chinese drawings too: GBK holds
        // 中, and does not hold ∅.
        let gbk = legacy_code_page("ANSI_936").unwrap();
        assert_eq!(gbk.encoding(), encoding_rs::GBK);
        assert!(gbk.is_faithful());
        assert_eq!(decode_mif_escapes("\\U+4E2D", gbk), "\\U+4E2D");
        assert_eq!(decode_mif_escapes("\\U+2205", gbk), "∅");
    }

    #[test]
    fn a_bare_encoding_names_no_code_page_and_decodes_unconditionally() {
        // Callers handed only an encoding know nothing about the drawing's
        // code page, and get the pre-gate behaviour rather than a guess.
        let bare: LegacyCodePage = encoding_rs::WINDOWS_1252.into();
        assert!(!bare.is_faithful());
        assert_eq!(decode_mif_escapes("94\\U+00B0", bare), "94°");
    }

    #[test]
    fn faithfulness_follows_the_row_the_table_actually_picks() {
        for name in [
            "ANSI_1250",
            "ANSI_1252",
            "ANSI_1258",
            "ANSI_874",
            "ISO8859-2",
            "ISO8859-15",
            "KOI8-R",
            "GB2312",
            "ANSI_936",
        ] {
            assert!(code_page_is_faithful(name), "{name} is its own encoder");
        }
        for name in [
            "DOS437",
            "DOS850",
            "DOS866",
            "MAC-ROMAN",
            "ANSI_932",
            "BIG5",
            "ANSI_949",
            "KOREAN",
            "ISO8859-1",
            "ISO8859-9",
            // WHATWG windows-1255 holds U+05BA where CP1255 does not, and
            // `encoding_rs`' `koi8-u` row is KOI8-RU, which differs from
            // KOI8-U in both directions.
            "ANSI_1255",
            "KOI8-U",
            "SOMETHING_UNKNOWN",
            // A spelling the table does not match reaches the windows-1252
            // default, so its encoder was not chosen for it.
            "ansi1252",
        ] {
            assert!(!code_page_is_faithful(name), "{name} is a stand-in");
        }
    }

    #[test]
    fn an_unrecognised_dwg_code_page_index_is_a_stand_in() {
        // `dwg_code_page_name` collapses every index outside the table onto the
        // recognised name `ANSI_1252`, so deriving faithfulness from the name
        // alone arms the gate on a drawing whose code page we never identified.
        // Index 0 is what the reader initialises the field to.
        for index in [0u16, 45, 99, 65535] {
            assert!(
                !encoding_from_dwg_code_page(index).is_faithful(),
                "index {index} is not in the table and must be a stand-in"
            );
        }
        // And the consequence the flag exists for: an escape a windows-1252
        // drawing would keep as content decodes on an unidentified code page,
        // because nothing established that the drawing could have held it.
        assert_eq!(
            decode_mif_escapes("94\\U+00B0", encoding_from_dwg_code_page(0)),
            "94°"
        );
        assert!(
            encoding_from_dwg_code_page(30).is_faithful(),
            "30 is ANSI_1252"
        );
        assert!(
            !encoding_from_dwg_code_page(23).is_faithful(),
            "23 is MAC-ROMAN, a stand-in"
        );
    }

    #[test]
    fn test_encode_legacy_string_mif_escapes() {
        // Mappable chars encode directly.
        assert_eq!(encode_legacy_string("AB", encoding_rs::WINDOWS_1252), b"AB");
        // Unmappable chars become MIF escapes, not HTML references.
        let encoded = encode_legacy_string("中", encoding_rs::WINDOWS_1252);
        assert_eq!(String::from_utf8(encoded).unwrap(), "\\U+4E2D");
        // Astral-plane chars become a surrogate pair of escapes.
        let encoded = encode_legacy_string("😀", encoding_rs::WINDOWS_1252);
        assert_eq!(String::from_utf8(encoded).unwrap(), "\\U+D83D\\U+DE00");
        // GBK encodes Chinese directly with no escapes.
        assert_eq!(encode_legacy_string("中", encoding_rs::GBK), &[0xD6, 0xD0]);
        // Round-trip through the decoder.
        let encoded = encode_legacy_string("a中b😀c", encoding_rs::WINDOWS_1252);
        let text = String::from_utf8(encoded).unwrap();
        assert_eq!(
            decode_mif_escapes(&text, legacy_code_page("ANSI_1252").unwrap()),
            "a中b😀c"
        );
    }

    #[test]
    fn test_legacy_round_trip_is_identity_where_no_literal_escape_names_an_unrepresentable_character(
    ) {
        // The claim has to be scoped, because the encoding is not injective
        // and no decoder can repair that: the character `∅` and the seven
        // literal characters `\U+2205` encode to the same bytes under
        // ANSI_1252, so whatever the decoder does, one of the two comes back
        // as the other. It resolves the collision towards transport, which is
        // the reading a conforming writer produces — and identity therefore
        // holds exactly for strings containing no literal escape that names a
        // character the code page cannot represent.
        let code_page = legacy_code_page("ANSI_1252").unwrap();
        let enc = code_page.encoding();
        // The lower-case row is the one that catches a decoder which
        // re-spells a content escape instead of copying it: `\U+00b0` is
        // seven representable characters and has to survive as written.
        for original in [
            "94\u{00B0}",
            "\u{2205}45,6",
            "94\\U+00B0",
            "94\\U+00b0",
            "a中b😀c",
            "\\U+D800",
        ] {
            let bytes = encode_legacy_string(original, enc);
            let (decoded, _, _) = enc.decode(&bytes);
            assert_eq!(decode_mif_escapes(&decoded, code_page), original);
        }
        // The excluded case, as a measurement rather than an identity
        // assertion it cannot satisfy.
        assert_eq!(
            encode_legacy_string("\\U+2205", enc),
            encode_legacy_string("\u{2205}", enc)
        );
    }
}
