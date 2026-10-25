//! Asking for AC1009 gives an R12 file (libviprs/acadsharp-rs-tests#111).
//!
//! Upstream's writer makes that an explicit request, `DxfWriter::with_version`,
//! because a document that merely carries AC1009 keeps being written as R13 so
//! nothing it holds is lost. These tests ask the way upstream wants them to.
//! Upstream has no binary R12 form either, so that test asserts the error it
//! returns instead of the one-byte-code layout the fork used to write.
//!
//! The ASCII and binary DXFs written for AC1009 and AC1012 used to be
//! byte-identical and the reader found AC1012 in both. R12 has no handles, no
//! `330` owners, no `100` subclass markers, no CLASSES or OBJECTS section, and
//! only the entities of its era; its binary form has one-byte group codes.

use std::io::Cursor;

use acadrust::entities::{Circle, EntityType, Line, LwPolyline, MText, Text};
use acadrust::types::{DxfVersion, Vector2, Vector3};
use acadrust::{CadDocument, DxfReader, DxfWriter};

fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    doc.add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 5.0, 5.0, 0.0)))
        .unwrap();
    doc.add_entity(EntityType::Circle(Circle::from_coords(1.0, 2.0, 0.0, 3.0)))
        .unwrap();
    doc.add_entity(EntityType::Text(Text::with_value(
        "hello",
        Vector3::new(1.0, 1.0, 0.0),
    )))
    .unwrap();
    let mut pl = LwPolyline::from_points(vec![
        Vector2::new(0.0, 0.0),
        Vector2::new(4.0, 0.0),
        Vector2::new(4.0, 4.0),
    ]);
    pl.close();
    doc.add_entity(EntityType::LwPolyline(pl)).unwrap();
    doc.add_entity(EntityType::MText(Box::new(MText::with_value(
        "first\\Psecond",
        Vector3::new(9.0, 9.0, 0.0),
    ))))
    .unwrap();
    doc
}

/// (code, value) pairs of an ASCII DXF.
fn pairs(dxf: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = dxf.lines().collect();
    lines
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| (c[0].trim().parse().unwrap(), c[1].trim().to_string()))
        .collect()
}

fn ascii(version: DxfVersion) -> String {
    String::from_utf8(
        DxfWriter::new(&document(version))
            .with_version(version)
            .write_to_vec()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn the_files_for_r12_and_r13_differ_and_say_which_they_are() {
    let r12 = ascii(DxfVersion::AC1009);
    let r13 = ascii(DxfVersion::AC1012);
    assert_ne!(r12, r13, "AC1009 and AC1012 must not be the same file");
    let p = pairs(&r12);
    let at = p.iter().position(|(c, v)| *c == 9 && v == "$ACADVER").unwrap();
    assert_eq!(p[at + 1], (1, "AC1009".to_string()));
}

#[test]
fn r12_has_no_handles_owners_or_subclass_markers() {
    let p = pairs(&ascii(DxfVersion::AC1009));
    // The only group 5 R12 may carry is none at all (not even $HANDSEED).
    for forbidden in [5, 100, 102, 330, 360] {
        assert!(
            !p.iter().any(|(c, _)| *c == forbidden),
            "group {forbidden} in an R12 file"
        );
    }
}

#[test]
fn r12_has_the_r12_sections_only_in_order() {
    let p = pairs(&ascii(DxfVersion::AC1009));
    let sections: Vec<&str> = p
        .windows(2)
        .filter(|w| w[0] == (0, "SECTION".to_string()) && w[1].0 == 2)
        .map(|w| w[1].1.as_str())
        .collect();
    assert_eq!(sections, ["HEADER", "TABLES", "BLOCKS", "ENTITIES"]);
    assert_eq!(p.last().unwrap(), &(0, "EOF".to_string()));
    // R12 does have a DIMSTYLE table (upstream writes one, as AutoCAD's own R12
    // files carry); BLOCK_RECORD is the R13 table.
    for table in ["BLOCK_RECORD"] {
        assert!(
            !p.contains(&(2, table.to_string())),
            "the {table} table is R13"
        );
    }
}

#[test]
fn r12_writes_the_polyline_in_its_heavy_form_and_text_for_mtext() {
    let p = pairs(&ascii(DxfVersion::AC1009));
    let entities: Vec<&str> = p
        .iter()
        .filter(|(c, _)| *c == 0)
        .map(|(_, v)| v.as_str())
        .collect();
    assert!(!entities.contains(&"LWPOLYLINE"));
    assert!(!entities.contains(&"MTEXT"));
    let at = entities.iter().position(|e| *e == "POLYLINE").unwrap();
    assert_eq!(
        &entities[at..at + 5],
        ["POLYLINE", "VERTEX", "VERTEX", "VERTEX", "SEQEND"]
    );
    let texts: Vec<&String> = p
        .iter()
        .skip_while(|(c, v)| !(*c == 2 && v == "ENTITIES"))
        .filter(|(c, _)| *c == 1)
        .map(|(_, v)| v)
        .collect();
    assert!(texts.iter().any(|t| *t == "first"));
    assert!(texts.iter().any(|t| *t == "second"));
}

#[test]
fn r12_reads_back_as_r12_with_its_geometry() {
    let bytes = DxfWriter::new(&document(DxfVersion::AC1009))
        .with_version(DxfVersion::AC1009)
        .write_to_vec()
        .unwrap();
    let doc = DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap();
    assert_eq!(doc.version, DxfVersion::AC1009);
    let lines = doc
        .entities()
        .filter(|e| matches!(e, EntityType::Line(_)))
        .count();
    let circles = doc
        .entities()
        .filter(|e| matches!(e, EntityType::Circle(_)))
        .count();
    assert_eq!((lines, circles), (1, 1));
}

#[test]
fn r12_binary_is_refused_rather_than_written_as_r13() {
    let doc = document(DxfVersion::AC1009);
    let err = DxfWriter::new_binary(&doc)
        .with_version(DxfVersion::AC1009)
        .write_to_vec()
        .expect_err("binary R12 is not written");
    assert!(
        err.to_string().contains("AC1009"),
        "the error says which version it refused: {err}"
    );
}

#[test]
fn r13_binary_is_unchanged_by_the_r12_mode() {
    let doc = document(DxfVersion::AC1012);
    let bytes = DxfWriter::new_binary(&doc).write_to_vec().unwrap();
    let sentinel = b"AutoCAD Binary DXF\r\n\x1a\x00";
    // `0 SECTION` starts with a two-byte code: 00 00.
    assert_eq!(&bytes[sentinel.len()..sentinel.len() + 2], [0, 0]);
}
