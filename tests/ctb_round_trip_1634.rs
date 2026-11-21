//! Round trip of a real CTB (libviprs/libviprs#1634): the text layer comes
//! back byte for byte, an unedited file comes back as it was read, and an
//! edit that is put back leaves the text as it was.

use acadrust::plot_style::PlotStyleTable;

const FIXTURE: &[u8] = include_bytes!("fixtures/ctb/sample_2024-LW0.ctb");
const NAME: &str = "sample_2024-LW0.ctb";

/// The stored text of a CTB: past the 60-byte header, inflated, NUL dropped.
fn stored_text(file: &[u8]) -> Vec<u8> {
    use std::io::Read;
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(&file[60..])
        .read_to_end(&mut out)
        .expect("inflate");
    assert_eq!(out.pop(), Some(0), "the stored text ends in a NUL");
    out
}

fn text_of(file: &[u8]) -> String {
    String::from_utf8(stored_text(file)).unwrap()
}

fn le32(file: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(file[at..at + 4].try_into().unwrap())
}

#[test]
fn ctb_sample_text_rewrite_is_byte_identical() {
    let table = PlotStyleTable::from_bytes(NAME, FIXTURE).unwrap();
    assert!(
        table.to_text() == text_of(FIXTURE),
        "text differs from the file"
    );
}

#[test]
fn ctb_sample_unedited_bytes_are_returned_as_read() {
    let table = PlotStyleTable::from_bytes(NAME, FIXTURE).unwrap();
    assert!(table.to_bytes().unwrap() == FIXTURE, "bytes differ");
}

#[test]
fn ctb_edit_aci_7_lineweight_then_revert_is_text_identical() {
    let mut table = PlotStyleTable::from_bytes(NAME, FIXTURE).unwrap();
    let original = table.aci_entries[7].lineweight;
    assert_eq!(original, 4);
    table.aci_entries[7].lineweight = 8;
    let edited = table.to_text();
    assert!(edited != text_of(FIXTURE));
    assert!(edited.contains("  lineweight=8\n"));
    table.aci_entries[7].lineweight = original;
    assert!(table.to_text() == text_of(FIXTURE), "reverted text differs");
    // the edit survives a save and a reload
    table.aci_entries[7].lineweight = 8;
    let reloaded = PlotStyleTable::from_bytes(NAME, &table.to_bytes().unwrap()).unwrap();
    assert_eq!(reloaded.aci_entries[7].lineweight, 8);
    assert!(reloaded.to_text() == edited);
}

#[test]
fn ctb_recompressed_header_is_consistent() {
    let mut table = PlotStyleTable::from_bytes(NAME, FIXTURE).unwrap();
    table.aci_entries[7].lineweight = 8;
    let bytes = table.to_bytes().unwrap();
    assert_eq!(&bytes[..48], &FIXTURE[..48]);
    let stream = &bytes[60..];
    assert_eq!(le32(&bytes, 48), adler32(stream));
    assert_eq!(le32(&bytes, 56) as usize, stream.len());
    assert_eq!(le32(&bytes, 52) as usize, stored_text(&bytes).len() + 1);
    // one digit swapped for another: the text is as long as the file's
    assert_eq!(le32(&bytes, 52), le32(FIXTURE, 52));
    assert!(table.load_warnings.is_empty());
    assert!(PlotStyleTable::from_bytes(NAME, &bytes)
        .unwrap()
        .load_warnings
        .is_empty());
}

#[test]
fn ctb_sample_entries_resolve() {
    let table = PlotStyleTable::from_bytes(NAME, FIXTURE).unwrap();
    for aci in (1..=9).chain([209, 251]) {
        let entry = &table.aci_entries[aci];
        assert_eq!(entry.color_policy, 5, "ACI {aci}");
        assert_eq!(table.resolve_color(aci as u8), Some([0.0; 3]), "ACI {aci}");
    }
    assert_eq!(table.aci_entries[7].lineweight, 4);
    assert_eq!(table.resolve_lineweight(7), Some(0.13));
    for aci in [10, 100, 255] {
        let entry = &table.aci_entries[aci];
        assert_eq!(table.resolve_color(aci as u8), None, "ACI {aci}");
        assert_eq!(table.resolve_screening(aci as u8), 1.0, "ACI {aci}");
        assert_eq!((entry.linetype, entry.color_policy), (31, 1), "ACI {aci}");
    }
}

#[test]
fn ctb_built_from_scratch_keeps_numeric_order() {
    let text = PlotStyleTable::identity("new.ctb").to_text();
    let first = text.find("plot_style{\n 0{\n").is_some();
    assert!(first);
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[test]
fn ctb_announced_length_over_cap_is_refused_before_inflating() {
    let mut file = FIXTURE.to_vec();
    file[52..56].copy_from_slice(&(5u32 * 1024 * 1024).to_le_bytes());
    // the stream is damaged too: the refusal has to come from the header
    file.truncate(70);
    let error = PlotStyleTable::from_bytes(NAME, &file).unwrap_err().to_string();
    assert!(error.contains("announces"), "{error}");
}
