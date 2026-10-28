//! #52: the DWG reader keeps the scale a file stores, even exactly 0.0.
//!
//! `tests/data/insert-zero-scale.dwg` holds one INSERT with scale (1, 0, 1) over a block with one line. No acadrust
//! writer can produce it (the setters rewrite anything under 1e-12), so it was written by LibreDWG's dwgwrite from
//! edited JSON. A DWG stores an INSERT scale as a plain double.
use acadrust::entities::EntityType;
use acadrust::{CadDocument, DwgReader};
use std::io::Cursor;

fn read(bytes: &[u8]) -> CadDocument {
    DwgReader::from_stream(Cursor::new(bytes.to_vec())).read().unwrap()
}

fn inserts(doc: &CadDocument) -> Vec<&acadrust::entities::Insert> {
    doc.entities()
        .filter_map(|e| if let EntityType::Insert(i) = e { Some(i) } else { None })
        .collect()
}

#[test]
fn a_stored_zero_scale_reads_back_as_zero() {
    let doc = read(include_bytes!("data/insert-zero-scale.dwg"));
    let ins = inserts(&doc);
    assert_eq!(ins.len(), 1);
    assert_eq!(ins[0].x_scale().to_bits(), 1.0f64.to_bits());
    assert_eq!(ins[0].y_scale().to_bits(), 0.0f64.to_bits(), "the file stores 0.0, the reader returned {:e}", ins[0].y_scale());
    assert_eq!(ins[0].z_scale().to_bits(), 1.0f64.to_bits());
}
