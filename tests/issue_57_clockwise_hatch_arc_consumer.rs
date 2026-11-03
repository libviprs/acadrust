//! #57: what a consumer of a clockwise HATCH arc edge gets from the model.
//!
//! AutoCAD stores the clockwise arc from 90 to 270 degrees (the right half of the circle) as
//! `73=0, 50=270, 51=90`. The suite adapter (acadsharp-rs-tests `src/acadrust_emit.rs`) turns an
//! edge into a counter-clockwise sweep by swapping start and end when `counter_clockwise` is
//! false, which only draws the right half if the model holds the endpoints as they sit on the
//! circle. Since upstream #242 the model keeps the stored angles, so the sweep is the left half.
use acadrust::document::CadDocument;
use acadrust::entities::hatch::{BoundaryEdge, BoundaryPath, BoundaryPathFlags, CircularArcEdge};
use acadrust::entities::{EntityType, Hatch};
use acadrust::io::dxf::{DxfReader, DxfWriter};
use acadrust::types::{DxfVersion, Vector2};
use std::f64::consts::{FRAC_PI_2, PI};
use std::io::Cursor;

/// A DXF whose only HATCH arc is the one AutoCAD writes for the clockwise arc 90 to 270.
fn autocad_dxf() -> Vec<u8> {
    let mut doc = CadDocument::with_version(DxfVersion::AC1015);
    let mut hatch = Hatch::solid();
    let mut flags = BoundaryPathFlags::new();
    flags.set_external(true);
    let mut path = BoundaryPath::with_flags(flags);
    path.edges.push(BoundaryEdge::CircularArc(CircularArcEdge {
        center: Vector2::new(0.0, 0.0),
        radius: 5.0,
        start_angle: 1.5 * PI,
        end_angle: FRAC_PI_2,
        counter_clockwise: true,
    }));
    hatch.add_path(path);
    doc.add_entity(EntityType::Hatch(Box::new(hatch))).unwrap();
    let bytes = DxfWriter::new(&doc).write_to_vec().unwrap();
    // Only the flag differs from the file AutoCAD writes: 73 = 0 for a clockwise edge.
    let text = String::from_utf8_lossy(&bytes).replace('\r', "");
    let at = text.find("\nHATCH\n").unwrap();
    let flag = text[at..].find("\n 73\n     1\n").unwrap() + at;
    format!("{}\n 73\n     0\n{}", &text[..flag], &text[flag + "\n 73\n     1\n".len()..]).into_bytes()
}

fn edge() -> CircularArcEdge {
    let doc = DxfReader::from_reader(Cursor::new(autocad_dxf())).unwrap().read().unwrap();
    for e in doc.entities() {
        if let EntityType::Hatch(h) = e {
            if let BoundaryEdge::CircularArc(a) = &h.paths[0].edges[0] {
                return a.clone();
            }
        }
    }
    panic!("no hatch arc edge")
}

/// The counter-clockwise sweep a consumer builds from the model, by the adapter's rule.
fn adapter_sweep(a: &CircularArcEdge) -> (f64, f64) {
    if a.counter_clockwise {
        (a.start_angle, a.end_angle)
    } else {
        (a.end_angle, a.start_angle)
    }
}

fn midpoint_x(start: f64, end: f64, radius: f64) -> f64 {
    let end = if end <= start { end + 2.0 * PI } else { end };
    (0.5 * (start + end)).cos() * radius
}

#[test]
fn the_file_edge_is_clockwise() {
    assert!(!edge().counter_clockwise);
}

#[test]
fn the_adapter_sweep_of_the_autocad_arc_is_the_right_half() {
    let a = edge();
    let (s, e) = adapter_sweep(&a);
    assert!(midpoint_x(s, e, a.radius) > 0.0, "sweep {s} to {e} is the left half");
}

#[test]
fn the_adapter_sweep_starts_at_270_degrees_and_ends_at_90() {
    let (s, e) = adapter_sweep(&edge());
    assert!((s - 1.5 * PI).abs() < 1e-9 && (e - FRAC_PI_2).abs() < 1e-9, "sweep {s} to {e}");
}
