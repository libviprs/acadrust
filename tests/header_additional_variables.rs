//! The header variables the DWG reader read and dropped, and the DXF reader
//! skipped, are fields of `HeaderVariables` (libviprs/acadsharp-rs-tests#102).

use acadrust::types::{Color, DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter, DxfReader};
use std::io::Cursor;

fn v(n: f64) -> Vector3 {
    Vector3::new(n, n + 1.0, n + 2.0)
}

fn set(doc: &mut CadDocument) {
    let h = &mut doc.header;
    h.create_ellipse_as_polyline = true;
    h.unit_mode = 1;
    h.dim_unit = 4;
    h.dim_fixed_ext_line_length = 1.5;
    h.dim_fixed_ext_line_on = true;
    h.dim_jog_angle = 0.5;
    h.dim_text_fill = 2;
    h.dim_text_fill_color = Color::from_index(5);
    h.dim_arc_symbol = 1;
    h.dim_text_direction = true;
    h.dim_alt_mzf = 12.5;
    h.dim_alt_mzs = "alt".to_string();
    h.dim_mzf = 7.5;
    h.dim_mzs = "mz".to_string();
    h.dwf_3d_precision = 4.0;
    h.polysolid_width = 0.5;
    h.polysolid_height = 3.0;
    h.light_glyph_display = 1;
    h.tile_model_light_synch = 0;
    h.dwf_frame = 2;
    h.dgn_frame = 1;
    h.cast_shadow = 3;
    h.interference_color = Color::from_index(3);
    for (i, o) in h.ucs_ortho_origins.iter_mut().enumerate() {
        *o = v(10.0 * (i as f64 + 1.0));
    }
    for (i, o) in h.paper_ucs_ortho_origins.iter_mut().enumerate() {
        *o = v(100.0 * (i as f64 + 1.0));
    }
}

fn roundtrip(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    set(&mut doc);
    let bytes = DwgWriter::write_to_vec(&doc).expect("write dwg");
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read dwg")
}

fn check_common(h: &acadrust::document::HeaderVariables, version: DxfVersion) {
    assert!(h.create_ellipse_as_polyline, "{version:?} PELLIPSE");
    assert_eq!(h.unit_mode, 1, "{version:?} UNITMODE");
    for (i, o) in h.ucs_ortho_origins.iter().enumerate() {
        assert_eq!(*o, v(10.0 * (i as f64 + 1.0)), "{version:?} UCSORG #{i}");
    }
    for (i, o) in h.paper_ucs_ortho_origins.iter().enumerate() {
        assert_eq!(*o, v(100.0 * (i as f64 + 1.0)), "{version:?} PUCSORG #{i}");
    }
}

#[test]
fn ac1015_orthographic_origins_and_modes() {
    let doc = roundtrip(DxfVersion::AC1015);
    check_common(&doc.header, DxfVersion::AC1015);
}

#[test]
fn ac1021_dimension_and_graphics_variables() {
    let doc = roundtrip(DxfVersion::AC1021);
    let h = &doc.header;
    check_common(h, DxfVersion::AC1021);
    assert_eq!(h.dim_fixed_ext_line_length, 1.5);
    assert!(h.dim_fixed_ext_line_on);
    assert_eq!(h.dim_jog_angle, 0.5);
    assert_eq!(h.dim_text_fill, 2);
    assert_eq!(h.dim_text_fill_color, Color::from_index(5));
    assert_eq!(h.dim_arc_symbol, 1);
    assert_eq!(h.dwf_3d_precision, 4.0);
    assert_eq!(h.polysolid_width, 0.5);
    assert_eq!(h.polysolid_height, 3.0);
    assert_eq!(h.light_glyph_display, 1);
    assert_eq!(h.tile_model_light_synch, 0);
    assert_eq!(h.dwf_frame, 2);
    assert_eq!(h.dgn_frame, 1);
    assert_eq!(h.cast_shadow, 3);
    assert_eq!(h.interference_color, Color::from_index(3));
}

#[test]
fn ac1032_zero_suppression_scale_and_direction() {
    let doc = roundtrip(DxfVersion::AC1032);
    let h = &doc.header;
    check_common(h, DxfVersion::AC1032);
    assert!(h.dim_text_direction);
    assert_eq!(h.dim_alt_mzf, 12.5);
    assert_eq!(h.dim_alt_mzs, "alt");
    assert_eq!(h.dim_mzf, 7.5);
    assert_eq!(h.dim_mzs, "mz");
}

#[test]
fn the_dxf_reader_takes_them_too() {
    let text = "  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1032\n\
  9\n$DIMFXL\n 40\n2.5\n  9\n$DIMFXLON\n290\n     1\n  9\n$DIMMZS\n  1\nxx\n\
  9\n$DIMTFILLCLR\n 62\n     6\n  9\n$PELLIPSE\n 70\n     1\n  9\n$CSHADOW\n280\n     2\n\
  9\n$DIMTXSTY\n  7\nMyDimText\n  9\n$DIMLTYPE\n  6\nBATTING\n  9\n$DIMLTEX1\n  6\nTRACKS\n\
  9\n$UCSORGTOP\n 10\n1.0\n 20\n2.0\n 30\n3.0\n  9\n$PUCSORGBACK\n 10\n4.0\n 20\n5.0\n 30\n6.0\n\
  0\nENDSEC\n  0\nEOF\n";
    let h = DxfReader::from_reader(Cursor::new(text.as_bytes().to_vec()))
        .expect("open")
        .read()
        .expect("read")
        .header;
    assert_eq!(h.dim_fixed_ext_line_length, 2.5);
    assert!(h.dim_fixed_ext_line_on);
    assert_eq!(h.dim_text_style_name, "MyDimText");
    assert_eq!(h.dim_linetype_name, "BATTING");
    assert_eq!(h.dim_linetype1_name, "TRACKS");
    assert_eq!(h.dim_mzs, "xx");
    assert_eq!(h.dim_text_fill_color, Color::from_index(6));
    assert!(h.create_ellipse_as_polyline);
    assert_eq!(h.cast_shadow, 2);
    assert_eq!(h.ucs_ortho_origins[0], Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(h.paper_ucs_ortho_origins[5], Vector3::new(4.0, 5.0, 6.0));
}
