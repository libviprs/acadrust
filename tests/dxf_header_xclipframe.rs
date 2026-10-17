//! $XCLIPFRAME is written by AutoCAD under group code 290, a bool, and the DXF
//! reader dropped it: it asked for an integer, got nothing, and left the
//! default (2), so a file that says 0 read as 2 (libviprs/acadsharp-rs-tests#100).

use acadrust::DxfReader;
use std::io::Cursor;

fn read(code: &str, value: &str) -> i16 {
    let text = format!(
        "  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1032\n  9\n$XCLIPFRAME\n{code}\n{value}\n  0\nENDSEC\n  0\nEOF\n"
    );
    DxfReader::from_reader(Cursor::new(text.into_bytes()))
        .expect("open")
        .read()
        .expect("read")
        .header
        .xclip_frame
}

#[test]
fn a_bool_zero_is_zero() {
    assert_eq!(read("290", "     0"), 0);
}

#[test]
fn a_bool_one_is_one() {
    assert_eq!(read("290", "     1"), 1);
}

#[test]
fn an_integer_under_280_is_still_read() {
    assert_eq!(read("280", "     2"), 2);
    assert_eq!(read("280", "     0"), 0);
}
