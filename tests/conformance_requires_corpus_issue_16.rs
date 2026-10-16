//! Issue #16: the conformance test has to fail, not pass, when it was meant to
//! compare and could not.
//!
//! `acadrust_matches_the_acadsharp_recording` returns early, and so reports
//! `ok`, whenever `ACADSHARP_CORPUS` is unset or points at a directory with no
//! `expectations/` in it. There is no way for a job to say the corpus is
//! required, so a fetch that failed, a typo in the path or an empty cache all
//! read as a pass.
//!
//! These tests run the real conformance test, not a copy of its logic: this
//! binary includes `acadsharp_conformance.rs` as a module, and each test
//! re-runs this same executable filtered to that one test, with the
//! environment under test, and looks at the exit status. That is the only
//! honest way to test "this test passes when it should not", because a test
//! cannot observe its own result.
#![cfg(unix)]

#[path = "acadsharp_conformance.rs"]
mod conformance;

use std::path::PathBuf;
use std::process::{Command, Output};

const CONFORMANCE: &str = "conformance::acadrust_matches_the_acadsharp_recording";

/// Run the included conformance test in a child process. `corpus` is the
/// value of `ACADSHARP_CORPUS` (unset when `None`), `require` sets
/// `ACADSHARP_REQUIRE_CORPUS=1`.
fn run_conformance(corpus: Option<&std::path::Path>, require: bool) -> Output {
    let exe = std::env::current_exe().expect("this test binary's own path");
    let mut cmd = Command::new(exe);
    cmd.args([CONFORMANCE, "--exact", "--nocapture", "--test-threads=1"]);
    cmd.env_remove("ACADSHARP_CORPUS");
    cmd.env_remove("ACADSHARP_REQUIRE_CORPUS");
    if let Some(p) = corpus {
        cmd.env("ACADSHARP_CORPUS", p);
    }
    if require {
        cmd.env("ACADSHARP_REQUIRE_CORPUS", "1");
    }
    cmd.output().expect("re-run this test binary")
}

fn joined(out: &Output) -> String {
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

fn ran_the_one_test(all: &str) {
    // Guard against the filter matching nothing, which would also exit 0 and
    // make every assertion below meaningless.
    assert!(
        all.contains("running 1 test"),
        "the child did not run exactly the conformance test; the filter `{CONFORMANCE}` is wrong:\n{all}"
    );
}

#[test]
fn issue_16_control_no_corpus_and_no_requirement_still_skips() {
    let out = run_conformance(None, false);
    let all = joined(&out);
    ran_the_one_test(&all);
    assert_eq!(
        out.status.code(),
        Some(0),
        "with neither ACADSHARP_CORPUS nor ACADSHARP_REQUIRE_CORPUS set, a plain `cargo test` \
         on a laptop has to stay green without a network fetch:\n{all}"
    );
    assert!(
        all.contains("skipping"),
        "the skip has to say it skipped:\n{all}"
    );
}

#[test]
fn issue_16_a_required_corpus_that_is_absent_fails_the_run() {
    let out = run_conformance(None, true);
    let all = joined(&out);
    ran_the_one_test(&all);
    assert_ne!(
        out.status.code(),
        Some(0),
        "ACADSHARP_REQUIRE_CORPUS=1 and no corpus, and the conformance test passed. A job that \
         says the comparison is required got a green run that compared nothing:\n{all}"
    );
    assert!(
        all.contains("ACADSHARP_REQUIRE_CORPUS"),
        "the failure does not name ACADSHARP_REQUIRE_CORPUS:\n{all}"
    );
}

#[test]
fn issue_16_a_corpus_path_that_holds_nothing_fails_the_run() {
    let empty = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("issue_16_empty_corpus");
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).unwrap();
    let out = run_conformance(Some(&empty), false);
    let all = joined(&out);
    ran_the_one_test(&all);
    assert_ne!(
        out.status.code(),
        Some(0),
        "ACADSHARP_CORPUS={} holds no expectations/, and the conformance test passed. Setting the \
         variable says the comparison was meant to run; pointing it at nothing is a typo or a \
         failed fetch, not a reason to skip:\n{all}",
        empty.display()
    );
    assert!(
        all.contains("ACADSHARP_CORPUS"),
        "the failure does not name ACADSHARP_CORPUS:\n{all}"
    );
}
