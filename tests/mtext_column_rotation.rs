//! A dynamic, manual-height column MTEXT keeps its rotation through DXF.
//!
//! Group 50 means the rotation on an MTEXT and, after the column marker 75,
//! the height of each column. A reader cannot tell them apart except by
//! position, so the rotation has to come first, and it has to be there even
//! when it is zero: without it the first column height is read as the
//! rotation (libviprs/acadsharp-rs-tests#113).

use std::io::Cursor;

use acadrust::entities::{EntityType, MText};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DxfReader, DxfWriter};

fn doc_with(rotation: f64) -> CadDocument {
    let mut m = MText::with_value("two\nlines", Vector3::new(1.0, 2.0, 0.0));
    m.rotation = rotation;
    m.column_data.column_type = 2;
    m.column_data.column_count = 1;
    m.column_data.auto_height = false;
    m.column_data.width = 15.0;
    m.column_data.gutter = 110.0;
    m.column_data.heights = vec![-2.9];
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_entity(EntityType::MText(m)).unwrap();
    doc
}

/// The (code, value) pairs of the first MTEXT after its AcDbMText marker.
fn mtext_pairs(dxf: &str) -> Vec<(i32, String)> {
    let lines: Vec<&str> = dxf.lines().collect();
    let mut pairs = Vec::new();
    let mut in_mtext = false;
    let mut i = 0;
    while i + 1 < lines.len() {
        let code: i32 = lines[i].trim().parse().unwrap();
        let value = lines[i + 1].trim().to_string();
        if code == 100 && value == "AcDbMText" {
            in_mtext = true;
        } else if in_mtext && code == 0 {
            break;
        } else if in_mtext {
            pairs.push((code, value));
        }
        i += 2;
    }
    pairs
}

fn read_back(bytes: Vec<u8>) -> MText {
    let doc = DxfReader::from_reader(Cursor::new(bytes))
        .unwrap()
        .read()
        .unwrap();
    let found = doc.entities().find_map(|e| match e {
        EntityType::MText(m) => Some(m.clone()),
        _ => None,
    });
    found.expect("the MTEXT survived")
}

#[test]
fn a_zero_rotation_is_written_before_the_column_heights() {
    let doc = doc_with(0.0);
    let text = String::from_utf8(DxfWriter::new(&doc).write_to_vec().unwrap()).unwrap();
    let pairs = mtext_pairs(&text);
    let first_50 = pairs.iter().position(|(c, _)| *c == 50).expect("a group 50");
    let marker_75 = pairs.iter().position(|(c, _)| *c == 75).expect("a group 75");
    assert!(
        first_50 < marker_75,
        "the first group 50 comes after the column marker, so a reader takes a column height for the rotation: {pairs:?}"
    );
    assert_eq!(pairs[first_50].1.parse::<f64>().unwrap(), 0.0);
}

#[test]
fn rotation_and_column_heights_survive_a_dxf_round_trip() {
    for rotation in [0.0, 0.5] {
        let doc = doc_with(rotation);
        let m = read_back(DxfWriter::new(&doc).write_to_vec().unwrap());
        assert!(
            (m.rotation - rotation).abs() < 1e-9,
            "rotation {rotation} came back as {}",
            m.rotation
        );
        assert_eq!(m.column_data.heights, vec![-2.9]);
    }
}
