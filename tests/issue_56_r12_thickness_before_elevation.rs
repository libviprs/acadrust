//! #56: an AC1009 entity stores its thickness before its elevation.
//!
//! LibreDWG (`src/decode.c`, `FLAG_R11_HAS_THICKNESS` then `FLAG_R11_HAS_ELEVATION`) and the
//! AC1009 Kaitai layout (`entity_thickness`, `entity_elevation`) agree on that order. Upstream
//! #275 swapped it in the reader and the writer.
use acadrust::{CadDocument, Circle, DwgReader, DwgWriter, DxfVersion, EntityType, Vector3};
use std::io::Cursor;

const THICKNESS: f64 = 2.0;
const ELEVATION: f64 = 3.0;

fn written() -> Vec<u8> {
    let mut document = CadDocument::with_version(DxfVersion::AC1009);
    let mut circle = Circle::from_coords(4.0, 5.0, ELEVATION, 6.0);
    circle.thickness = THICKNESS;
    document.add_entity(EntityType::Circle(circle)).unwrap();
    DwgWriter::write_to_vec(&document).unwrap()
}

fn pair(first: f64, second: f64) -> Vec<u8> {
    [first.to_le_bytes(), second.to_le_bytes()].concat()
}

fn find(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    bytes.windows(needle.len()).position(|w| w == needle)
}

fn read_circle(bytes: Vec<u8>) -> Circle {
    let document = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let circle = document
        .entities()
        .find_map(|e| match e {
            EntityType::Circle(c) => Some(Circle::clone(c)),
            _ => None,
        })
        .expect("a circle");
    circle
}

#[test]
fn the_writer_puts_thickness_before_elevation() {
    let bytes = written();
    assert!(
        find(&bytes, &pair(THICKNESS, ELEVATION)).is_some(),
        "no thickness-then-elevation pair in the record (elevation-then-thickness present: {})",
        find(&bytes, &pair(ELEVATION, THICKNESS)).is_some()
    );
}

#[test]
fn the_writer_does_not_put_elevation_before_thickness() {
    assert!(find(&written(), &pair(ELEVATION, THICKNESS)).is_none());
}

#[test]
fn the_reader_takes_the_first_double_as_thickness() {
    // The record as LibreDWG and the Kaitai layout lay it out: thickness 2, then elevation 3.
    let mut bytes = written();
    if let Some(at) = find(&bytes, &pair(ELEVATION, THICKNESS)) {
        bytes[at..at + 16].copy_from_slice(&pair(THICKNESS, ELEVATION));
    }
    let circle = read_circle(bytes);
    assert_eq!(circle.thickness, THICKNESS, "thickness");
    assert_eq!(circle.center.z, ELEVATION, "elevation");
}

#[test]
fn a_round_trip_keeps_both_values() {
    let circle = read_circle(written());
    assert_eq!((circle.thickness, circle.center.z), (THICKNESS, ELEVATION));
    assert_eq!(circle.center, Vector3::new(4.0, 5.0, ELEVATION));
}
