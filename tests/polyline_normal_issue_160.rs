//! The extrusion of a 2D polyline survives the DXF writer (libviprs/acadsharp-rs-tests#160).
//!
//! The LWPOLYLINE form wrote its normal (210/220/230), and the heavy POLYLINE form never did,
//! so a polyline in an OCS plane came back with the world normal below AC1015 (where there is
//! no LWPOLYLINE to down-save to) and for a spline-fit polyline at every version.

use acadrust::document::CadDocument;
use acadrust::entities::{EntityType, Polyline2D, PolylineFlags, Vertex2D};
use acadrust::io::dxf::{DxfReader, DxfWriter};
use acadrust::types::{DxfVersion, Vector3};
use std::io::Cursor;

fn flipped() -> Polyline2D {
    let mut pl = Polyline2D::new();
    pl.elevation = 2.0;
    pl.normal = Vector3::new(0.0, 0.0, -1.0);
    pl.add_vertex(Vertex2D::new(Vector3::new(20.0, 0.0, 0.0)));
    pl.add_vertex(Vertex2D::new(Vector3::new(30.0, 5.0, 0.0)));
    pl.add_vertex(Vertex2D::new(Vector3::new(40.0, 0.0, 0.0)));
    pl
}

fn round_trip(pl: Polyline2D, version: DxfVersion) -> (String, Vec<Vector3>) {
    let mut doc = CadDocument::with_version(version);
    doc.add_entity(EntityType::Polyline2D(pl)).unwrap();
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    let text = String::from_utf8_lossy(&bytes).replace('\r', "");
    let back = DxfReader::from_reader(Cursor::new(bytes)).unwrap().read().unwrap();
    let normals = back
        .entities()
        .filter_map(|e| match e {
            EntityType::Polyline2D(p) => Some(p.normal),
            EntityType::LwPolyline(p) => Some(p.normal),
            _ => None,
        })
        .collect();
    (text, normals)
}

fn polyline_record(text: &str) -> &str {
    let at = text.find("\nPOLYLINE\n").expect("a POLYLINE record");
    let rest = &text[at + 1..];
    &rest[..rest.find("SEQEND").expect("its SEQEND")]
}

#[test]
fn issue_160_a_heavy_polyline_below_r2000_keeps_its_normal() {
    for version in [DxfVersion::AC1009, DxfVersion::AC1012, DxfVersion::AC1014] {
        let (text, normals) = round_trip(flipped(), version);
        let record = polyline_record(&text);
        let count = record.matches("\n230\n-1.0\n").count() + record.matches("\n230\n-1\n").count();
        assert_eq!(count, 1, "{version:?}: the POLYLINE record has {count} 230 -1 groups, not one");
        assert_eq!(normals, vec![Vector3::new(0.0, 0.0, -1.0)], "{version:?}");
    }
}

#[test]
fn issue_160_a_spline_fit_polyline_keeps_its_normal_at_every_version() {
    for version in [DxfVersion::AC1014, DxfVersion::AC1015, DxfVersion::AC1032] {
        let mut pl = flipped();
        pl.flags = pl.flags | PolylineFlags::SPLINE_FIT;
        let (text, normals) = round_trip(pl, version);
        assert!(text.contains("\nPOLYLINE\n"), "{version:?}: kept in its heavy form");
        assert_eq!(normals, vec![Vector3::new(0.0, 0.0, -1.0)], "{version:?}");
    }
}

#[test]
fn issue_160_the_default_normal_is_still_not_written() {
    let mut pl = flipped();
    pl.normal = Vector3::UNIT_Z;
    let (text, normals) = round_trip(pl, DxfVersion::AC1014);
    assert!(!polyline_record(&text).contains("\n230\n"), "a world normal is the default");
    assert_eq!(normals, vec![Vector3::UNIT_Z]);
}

#[test]
fn issue_160_the_lwpolyline_down_save_keeps_its_normal() {
    let (text, normals) = round_trip(flipped(), DxfVersion::AC1032);
    assert!(text.contains("\nLWPOLYLINE\n"));
    assert_eq!(normals, vec![Vector3::new(0.0, 0.0, -1.0)]);
}
