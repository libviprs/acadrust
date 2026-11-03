//! #58: the API the pinned suite adapter (acadsharp-rs-tests at `ACADRUST_REV` 72af489) calls.
//!
//! Every line below is copied from `src/acadrust_model1.rs` and `tests/emit_acadrust.rs` of the
//! suite (it also matches `ObjectType` without an `Arch` arm, which cannot be kept here without
//! listing every variant). On `main` this file does not compile, which is the red state: it lists what a pin bump
//! has to change. It is not meant to be merged.
use acadrust::entities::{EntityType, MText, Viewport};
use acadrust::types::Vector3;
use acadrust::CadDocument;

#[test]
fn header_fields_the_adapter_reads() {
    let doc = CadDocument::new();
    let h = &doc.header;
    let _ = (
        h.polyline_ellipse,
        h.interfere_color.clone(),
        h.dim_fixed_ext_length,
        h.dim_fixed_ext_length_on,
    );
}

#[test]
fn entity_variants_take_the_value_unboxed() {
    let mut doc = CadDocument::new();
    doc.add_entity(EntityType::Viewport(Viewport::new())).unwrap();
    let m = MText::with_value("block", Vector3::new(0.0, 0.0, 0.0));
    doc.add_entity(EntityType::MText(m)).unwrap();
}
