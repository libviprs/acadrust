//! Writing the same document twice must produce the same bytes.
//!
//! `CadDocument::objects` is a `HashMap`, and every map hashes with its own
//! random state, so iterating it directly makes the output depend on the
//! iteration order of the moment. The writers sort by handle wherever they
//! walk that map: the DWG object writer for the orphaned objects it drains
//! in its second phase, the DXF section writer for the OBJECTS section, and
//! the "Standard" MLINESTYLE fallback in the header. These tests pin that
//! down with enough orphaned objects that an unsorted walk cannot pass by
//! luck.

use acadrust::objects::{Dictionary, ObjectType};
use acadrust::{CadDocument, DwgWriter, DxfWriter};

/// Dictionaries that no other object owns: exactly the objects the DWG
/// writer only reaches in its second phase, and a permutation the DXF
/// OBJECTS section shows in full.
const ORPHANED_DICTIONARIES: usize = 32;

fn document_with_orphaned_objects() -> CadDocument {
    let mut document = CadDocument::new();

    for index in 0..ORPHANED_DICTIONARIES {
        let handle = document.allocate_handle();
        let mut dictionary = Dictionary::new();
        dictionary.handle = handle;
        dictionary.add_entry(format!("ENTRY_{index}"), handle);
        document
            .objects
            .insert(handle, ObjectType::Dictionary(dictionary));
    }

    document
}

fn assert_three_runs_are_byte_identical(label: &str, write: impl Fn() -> Vec<u8>) {
    let first = write();

    for run in 2..=3 {
        let again = write();
        assert_eq!(
            first.len(),
            again.len(),
            "{label}: run {run} has a different length"
        );

        if let Some(offset) = first.iter().zip(&again).position(|(a, b)| a != b) {
            panic!("{label}: run {run} differs at byte offset {offset}");
        }
    }
}

#[test]
fn dxf_output_is_byte_identical_across_runs() {
    assert_three_runs_are_byte_identical("DXF", || {
        let document = document_with_orphaned_objects();
        DxfWriter::new(&document).write_to_vec().expect("DXF write")
    });
}

#[test]
fn dwg_output_is_byte_identical_across_runs() {
    assert_three_runs_are_byte_identical("DWG", || {
        let document = document_with_orphaned_objects();
        DwgWriter::write_to_vec(&document).expect("DWG write")
    });
}
