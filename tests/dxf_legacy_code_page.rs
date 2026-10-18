//! A pre-2007 DXF is in the code page its `$DWGCODEPAGE` names (libviprs/acadsharp-rs-tests#152
//! for the writer, #154 for the reader). Characters the page cannot hold are MIF `\U+XXXX`
//! escapes, as in the DWG writer. AC1021 and later are UTF-8 and stay so.

use acadrust::document::CadDocument;
use acadrust::entities::{EntityType, Text};
use acadrust::io::dxf::{DxfReader, DxfWriter};
use acadrust::types::{DxfVersion, Vector3};
use std::io::Cursor;

const HELD_1250: &str = "ĄĆĘŁŃÓŚŹŻ ąćęłńóśźż";
const HELD_1250_BYTES: &[u8] =
    b"\xa5\xc6\xca\xa3\xd1\xd3\x8c\x8f\xaf \xb9\xe6\xea\xb3\xf1\xf3\x9c\x9f\xbf";

fn doc_with(texts: &[&str], page: &str, version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::new();
    doc.version = version;
    doc.header.code_page = page.to_string();
    for (i, t) in texts.iter().enumerate() {
        doc.add_entity(EntityType::Text(
            Text::with_value(t.to_string(), Vector3::new(0.0, 10.0 * i as f64, 0.0))
                .with_height(2.5),
        ))
        .unwrap();
    }
    doc
}

fn texts_of(doc: &CadDocument) -> Vec<String> {
    doc.entities()
        .filter_map(|e| match e {
            EntityType::Text(t) => Some(t.value.clone()),
            _ => None,
        })
        .collect()
}

fn read(bytes: &[u8]) -> CadDocument {
    DxfReader::from_reader(Cursor::new(bytes.to_vec()))
        .unwrap()
        .read()
        .unwrap()
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn issue_152_a_pre_2007_dxf_is_written_in_its_code_page() {
    let doc = doc_with(&[HELD_1250], "ANSI_1250", DxfVersion::AC1015);
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    assert!(
        contains(&bytes, HELD_1250_BYTES),
        "the held text should be the ANSI_1250 bytes"
    );
    assert!(
        !contains(&bytes, HELD_1250.as_bytes()),
        "and not the UTF-8 bytes under a legacy page name"
    );
}

#[test]
fn issue_152_a_character_the_page_cannot_hold_is_a_mif_escape() {
    let doc = doc_with(&["\u{2205}45,6"], "ANSI_1250", DxfVersion::AC1015);
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    assert!(
        contains(&bytes, b"\\U+220545,6"),
        "{}",
        String::from_utf8_lossy(&bytes)
    );
}

#[test]
fn issue_152_binary_dxf_follows_the_same_rule() {
    let doc = doc_with(
        &[HELD_1250, "\u{2205}45,6"],
        "ANSI_1250",
        DxfVersion::AC1015,
    );
    let bytes = DxfWriter::new_binary(&doc).write_to_vec().unwrap();
    assert!(contains(&bytes, HELD_1250_BYTES));
    assert!(contains(&bytes, b"\\U+220545,6"));
}

#[test]
fn issue_152_ac1021_and_later_stay_utf8() {
    let doc = doc_with(&[HELD_1250], "ANSI_1250", DxfVersion::AC1021);
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    assert!(contains(&bytes, HELD_1250.as_bytes()));
}

#[test]
fn issue_152_every_page_round_trips_through_acadrust() {
    for (page, held) in [
        ("ANSI_1250", HELD_1250),
        ("ANSI_1251", "АБВГДЕЖЗ абвгдежз"),
        ("ANSI_932", "日本語テスト"),
        ("ANSI_936", "中文测试"),
        ("ANSI_949", "한국어"),
        ("ANSI_950", "中文測試"),
        ("ANSI_874", "กขคงจ"),
    ] {
        for ver in [DxfVersion::AC1015, DxfVersion::AC1018] {
            let doc = doc_with(&[held, "\u{2205}45,6"], page, ver);
            let back = texts_of(&read(&DxfWriter::new(&doc).write_to_vec().unwrap()));
            assert_eq!(
                back,
                vec![held.to_string(), "\u{2205}45,6".to_string()],
                "{page} {ver:?}"
            );
        }
    }
}

fn minimal_dxf(page: &str, text: &[u8]) -> Vec<u8> {
    minimal_dxf_ordered(page, text, false)
}

/// `page_first`: $DWGCODEPAGE before $ACADVER, as ACadSharp writes them.
fn minimal_dxf_ordered(page: &str, text: &[u8], page_first: bool) -> Vec<u8> {
    let mut v = Vec::new();
    let ver = "  9\r\n$ACADVER\r\n  1\r\nAC1015\r\n";
    let cp = format!("  9\r\n$DWGCODEPAGE\r\n  3\r\n{page}\r\n");
    let vars = if page_first {
        format!("{cp}{ver}")
    } else {
        format!("{ver}{cp}")
    };
    let head = format!(
        "  0\r\nSECTION\r\n  2\r\nHEADER\r\n{vars}  0\r\nENDSEC\r\n  0\r\nSECTION\r\n  2\r\nENTITIES\r\n  0\r\nTEXT\r\n  5\r\n20\r\n  8\r\n0\r\n 10\r\n0.0\r\n 20\r\n0.0\r\n 30\r\n0.0\r\n 40\r\n2.5\r\n  1\r\n"
    );
    v.extend_from_slice(head.as_bytes());
    v.extend_from_slice(text);
    v.extend_from_slice(b"\r\n  0\r\nENDSEC\r\n  0\r\nEOF\r\n");
    v
}

#[test]
fn issue_154_the_reader_decodes_a_foreign_dxf_by_its_header_page() {
    // What ACadSharp writes: the bytes of the page, no escape.
    let doc = read(&minimal_dxf("ANSI_1250", HELD_1250_BYTES));
    assert_eq!(texts_of(&doc), vec![HELD_1250.to_string()]);
}

#[test]
fn issue_154_the_reader_decodes_mif_escapes_of_what_the_page_cannot_hold() {
    let doc = read(&minimal_dxf("ANSI_1250", b"\\U+220545,6"));
    assert_eq!(texts_of(&doc), vec!["\u{2205}45,6".to_string()]);
}

#[test]
fn issue_154_an_escape_the_page_could_have_held_stays_literal() {
    // The code page gate of libviprs/acadrust#9: 94\U+00B0 at ANSI_1250 is content.
    let doc = read(&minimal_dxf("ANSI_1250", b"94\\U+00B0"));
    assert_eq!(texts_of(&doc), vec!["94\\U+00B0".to_string()]);
}

#[test]
fn issue_154_the_page_counts_when_it_comes_before_the_version() {
    // ACadSharp's header has $DWGCODEPAGE ahead of $ACADVER.
    let doc = read(&minimal_dxf_ordered("ANSI_1250", HELD_1250_BYTES, true));
    assert_eq!(texts_of(&doc), vec![HELD_1250.to_string()]);
    let doc = read(&minimal_dxf_ordered("ANSI_1250", b"\\U+220545,6", true));
    assert_eq!(texts_of(&doc), vec!["\u{2205}45,6".to_string()]);
}
