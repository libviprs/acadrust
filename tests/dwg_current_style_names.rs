//! The current text style, linetype and dimension style of a DWG come back as
//! names, the way the DXF reader gives them.
//!
//! The DWG header holds each of them as a handle. The reader resolved the
//! current layer and left the other three at their defaults ("Standard" and
//! "ByLayer"), so a drawing whose current dimension style is not Standard read
//! as Standard (libviprs/acadsharp-rs-tests#98).

use acadrust::tables::{BlockRecord, DimStyle, LineType, TextStyle};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter, TableEntry};

fn document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);

    let mut text = TextStyle::new("MyText");
    text.set_handle(doc.allocate_handle());
    doc.header.current_text_style_handle = text.handle;
    doc.text_styles.add(text).unwrap();

    let mut dim = DimStyle::new("MyDim");
    dim.set_handle(doc.allocate_handle());
    doc.header.current_dimstyle_handle = dim.handle;
    doc.dim_styles.add(dim).unwrap();

    let mut lt = LineType::new("MyDash");
    lt.set_handle(doc.allocate_handle());
    doc.header.current_linetype_handle = lt.handle;
    doc.line_types.add(lt).unwrap();

    doc
}

fn roundtrip(version: DxfVersion) {
    let bytes = DwgWriter::write_to_vec(&document(version)).expect("write dwg");
    let loaded = DwgReader::from_stream(std::io::Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let h = &loaded.header;
    assert_eq!(h.current_dimstyle_name, "MyDim", "{version:?} $DIMSTYLE");
    assert_eq!(h.current_text_style_name, "MyText", "{version:?} $TEXTSTYLE");
    assert_eq!(h.current_linetype_name, "MyDash", "{version:?} $CELTYPE");
}

#[test]
fn ac1015_current_styles_are_names() {
    roundtrip(DxfVersion::AC1015);
}

#[test]
fn ac1018_current_styles_are_names() {
    roundtrip(DxfVersion::AC1018);
}

#[test]
fn ac1032_current_styles_are_names() {
    roundtrip(DxfVersion::AC1032);
}

/// $DIMBLK and its siblings: the DWG header holds a handle, the DXF holds the
/// arrowhead's name, and a standard arrowhead's block is that name with an
/// underscore in front (libviprs/acadsharp-rs-tests#100).
fn arrow_document(version: DxfVersion) -> CadDocument {
    let mut doc = CadDocument::with_version(version);
    for name in ["_BoxBlank", "_Dot", "MyArrow"] {
        let mut br = BlockRecord::new(name);
        br.set_handle(doc.allocate_handle());
        br.block_entity_handle = doc.allocate_handle();
        br.block_end_handle = doc.allocate_handle();
        doc.block_records.add(br).unwrap();
    }
    doc.header.dim_arrow_block = "BoxBlank".to_string();
    doc.header.dim_arrow_block1 = "MyArrow".to_string();
    doc.header.dim_arrow_block2 = "Dot".to_string();
    doc.header.dim_leader_arrow_block = "Dot".to_string();
    doc
}

fn arrow_roundtrip(version: DxfVersion) {
    let bytes = DwgWriter::write_to_vec(&arrow_document(version)).expect("write dwg");
    let loaded = DwgReader::from_stream(std::io::Cursor::new(bytes))
        .read()
        .expect("read dwg");
    let h = &loaded.header;
    assert_eq!(h.dim_arrow_block, "BoxBlank", "{version:?} $DIMBLK");
    assert_eq!(h.dim_arrow_block1, "MyArrow", "{version:?} $DIMBLK1");
    assert_eq!(h.dim_arrow_block2, "Dot", "{version:?} $DIMBLK2");
    assert_eq!(h.dim_leader_arrow_block, "Dot", "{version:?} $DIMLDRBLK");
}

#[test]
fn ac1015_arrow_blocks_are_names() {
    arrow_roundtrip(DxfVersion::AC1015);
}

#[test]
fn ac1018_arrow_blocks_are_names() {
    arrow_roundtrip(DxfVersion::AC1018);
}

#[test]
fn ac1032_arrow_blocks_are_names() {
    arrow_roundtrip(DxfVersion::AC1032);
}
