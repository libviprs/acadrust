//! Issue #9: three places where the MIF code-page gate does not hold the
//! promise its own names make.
//!
//! Public API only, no fixtures and no corpus.
//!
//! 1. `DOS866`, `KOREAN` and `ANSI_949` are their own `encoding_rs` row
//!    (`ibm866` is CP866, `euc-kr` is CP949), but are pinned as stand-ins, so
//!    a literal `\U+00B0` in a Russian OEM or Korean drawing is decoded as if
//!    a writer had escaped it. `ANSI_1251` is the control: same input, a
//!    faithful row, the escape stays literal. `DOS855` (CP855 is not CP866)
//!    and `JOHAB` (CP1361 is not CP949) really are stand-ins and stay that
//!    way.
//! 2. `encode_legacy_string` escapes a character only when the encoder says it
//!    is unmappable. WHATWG Shift_JIS encodes `U+00A5` to a bare `0x5C`, so
//!    the writer can manufacture a backslash, and with it an escape, out of
//!    drawing content.
//!
//! (The third item in the issue, GBK against CP936, is a 178-scalar
//! repertoire gap recorded as accepted: a comment says so, and a test would
//! only pin the table.)

use acadrust::io::dxf::code_page::{
    code_page_is_faithful, decode_cif_escapes_in, encode_legacy_string, encoding_from_code_page,
};

#[test]
fn issue_9_dos866_korean_and_ansi_949_are_their_own_encoder() {
    for name in ["DOS866", "KOREAN", "ANSI_949"] {
        assert!(
            code_page_is_faithful(name),
            "{name} resolves to an encoding_rs row that IS that code page, so it is faithful"
        );
        let page = encoding_from_code_page(name).unwrap();
        assert!(page.is_faithful(), "{name}: LegacyCodePage disagrees");
        assert_eq!(
            decode_cif_escapes_in("94\\U+00B0", page),
            "94\\U+00B0",
            "{name}: a representable escape is drawing content and must stay literal, as it does \
             under ANSI_1251"
        );
    }
}

#[test]
fn issue_9_control_ansi_1251_keeps_the_escape_literal() {
    let page = encoding_from_code_page("ANSI_1251").unwrap();
    assert!(page.is_faithful());
    assert_eq!(decode_cif_escapes_in("94\\U+00B0", page), "94\\U+00B0");
}

#[test]
fn issue_9_dos855_and_johab_are_exact_tables_here() {
    // The gate first pinned these as stand-ins, because the only rows
    // available were IBM866 (for CP855) and EUC-KR (for CP1361). Upstream has
    // since given both a table of their own, so they are faithful, and an
    // escape naming a character the table cannot hold is still transport.
    for name in ["DOS855", "JOHAB"] {
        assert!(code_page_is_faithful(name), "{name} has its own table");
        let page = encoding_from_code_page(name).unwrap();
        assert!(page.is_faithful(), "{name}");
        assert_eq!(decode_cif_escapes_in("\\U+0E01", page), "\u{0E01}", "{name}");
    }
}

#[test]
fn issue_9_the_shift_jis_writer_does_not_turn_a_yen_sign_into_a_backslash() {
    let enc = encoding_rs::SHIFT_JIS;
    let bytes = encode_legacy_string("\u{A5}", enc);
    assert_ne!(
        bytes,
        b"\\",
        "U+00A5 was written as a bare 0x5C, which reads back as a backslash"
    );
    // The escape the issue shows: authored "¥U+0041" must not read back as "A".
    let authored = "\u{A5}U+0041";
    let bytes = encode_legacy_string(authored, enc);
    let (on_disk, _, _) = enc.decode(&bytes);
    let page = encoding_from_code_page("ANSI_932").unwrap();
    assert_eq!(
        decode_cif_escapes_in(&on_disk, page),
        authored,
        "seven characters collapsed to one on a plain write/read"
    );
}

#[test]
fn issue_9_every_character_shift_jis_does_not_round_trip_is_escaped() {
    // The module's premise is that an escape in a file is never a character
    // the code page could hold. That only follows if the writer escapes every
    // character that does not survive the encoder, not just the ones it
    // reports as unmappable. U+203E (overline) encodes to 0x7E, which reads
    // back as a tilde, and U+00A5 to 0x5C.
    let enc = encoding_rs::SHIFT_JIS;
    let page = encoding_from_code_page("ANSI_932").unwrap();
    for ch in ['\u{A5}', '\u{203E}', '\u{2212}', 'A', '\u{3042}', '\u{4E2D}'] {
        let s = ch.to_string();
        let bytes = encode_legacy_string(&s, enc);
        let (on_disk, _, _) = enc.decode(&bytes);
        assert_eq!(
            decode_cif_escapes_in(&on_disk, page),
            s,
            "U+{:04X} did not survive a write/read through Shift_JIS",
            ch as u32
        );
    }
}
