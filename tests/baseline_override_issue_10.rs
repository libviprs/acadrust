//! Issue #10: the conformance verdict check must be able to compare against an
//! expectation the change under test cannot edit.
//!
//! `acadsharp_conformance.rs` bakes `tests/acadsharp/BASELINE.tsv` into the
//! binary with `include_str!`, so the pull request under test owns its own
//! expectation: regress a fixture, re-record, commit both files, and CI is
//! green. Part of the repair is that the test can read the baseline from a
//! file named by `ACADSHARP_BASELINE` instead, which CI points at the base
//! branch's copy on a pull request. The other part is that workflow step.
//!
//! These tests run the real conformance test, as `conformance_requires_corpus_issue_16.rs`
//! does: this binary includes it as a module and re-runs itself filtered to
//! that one test, with the environment under test. They need the corpus, so
//! they take it from `ACADSHARP_CORPUS` like the conformance test does, skip
//! out loud without it, and fail without it when `ACADSHARP_REQUIRE_CORPUS=1`.
#![cfg(unix)]

#[path = "acadsharp_conformance.rs"]
mod conformance;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CONFORMANCE: &str = "conformance::acadrust_matches_the_acadsharp_recording";

fn corpus() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var("ACADSHARP_CORPUS").ok().filter(|v| !v.is_empty())?);
    p.join("expectations").is_dir().then_some(p)
}

/// The corpus these tests need, or `None` after saying so. With
/// `ACADSHARP_REQUIRE_CORPUS=1` a missing corpus is a failure.
fn need_corpus(test: &str) -> Option<PathBuf> {
    let c = corpus();
    if c.is_none() {
        let msg = format!("{test}: no corpus (set ACADSHARP_CORPUS), so the baseline override was NOT checked");
        if std::env::var("ACADSHARP_REQUIRE_CORPUS").as_deref() == Ok("1") {
            panic!("{msg}, and ACADSHARP_REQUIRE_CORPUS=1 says it is required");
        }
        eprintln!("{msg}");
    }
    c
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_with_baseline(corpus: &Path, baseline: &Path) -> Output {
    let exe = std::env::current_exe().expect("this test binary's own path");
    Command::new(exe)
        .args([CONFORMANCE, "--exact", "--nocapture", "--test-threads=1"])
        .env("ACADSHARP_CORPUS", corpus)
        .env("ACADSHARP_BASELINE", baseline)
        .output()
        .expect("re-run this test binary")
}

fn joined(out: &Output) -> String {
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

fn committed_baseline() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/acadsharp/BASELINE.tsv");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn issue_10_control_an_unmodified_baseline_passes() {
    let Some(corpus) = need_corpus("issue_10_control") else {
        return;
    };
    let dir = scratch("issue_10_control");
    let base = dir.join("base.tsv");
    std::fs::write(&base, committed_baseline()).unwrap();
    let out = run_with_baseline(&corpus, &base);
    let all = joined(&out);
    assert!(all.contains("running 1 test"), "wrong filter:\n{all}");
    assert_eq!(out.status.code(), Some(0), "{all}");
}

#[test]
fn issue_10_a_baseline_from_elsewhere_that_disagrees_fails_the_run() {
    let Some(corpus) = need_corpus("issue_10_disagree") else {
        return;
    };
    // The base branch's recording says a fixture was COUNT:1/2, the run says
    // MATCH. A PR that regressed something and re-recorded would look like
    // this from the base branch's side.
    let committed = committed_baseline();
    let (name, edited) = {
        let mut name = String::new();
        let mut out = String::new();
        for line in committed.lines() {
            match line.split_once('\t') {
                Some((n, "MATCH")) if name.is_empty() => {
                    name = n.to_string();
                    out.push_str(&format!("{n}\tCOUNT:1/2\n"));
                }
                _ => {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        assert!(!name.is_empty(), "BASELINE.tsv has no MATCH row to edit");
        (name, out)
    };
    let dir = scratch("issue_10_disagree");
    let base = dir.join("base.tsv");
    std::fs::write(&base, edited).unwrap();
    let out = run_with_baseline(&corpus, &base);
    let all = joined(&out);
    assert!(all.contains("running 1 test"), "wrong filter:\n{all}");
    assert_ne!(
        out.status.code(),
        Some(0),
        "ACADSHARP_BASELINE names a recording that disagrees with the run on {name}, and the \
         test passed: it compared against the baseline baked into the binary instead:\n{all}"
    );
    assert!(all.contains(&name), "the failure does not name {name}:\n{all}");
}

#[test]
fn issue_10_a_baseline_path_that_does_not_exist_fails_the_run() {
    let Some(corpus) = need_corpus("issue_10_missing") else {
        return;
    };
    let dir = scratch("issue_10_missing");
    let out = run_with_baseline(&corpus, &dir.join("no-such-baseline.tsv"));
    let all = joined(&out);
    assert!(all.contains("running 1 test"), "wrong filter:\n{all}");
    assert_ne!(
        out.status.code(),
        Some(0),
        "ACADSHARP_BASELINE points at a file that is not there and the test passed anyway; \
         naming a baseline is a statement that you meant to compare against it:\n{all}"
    );
    assert!(
        all.contains("ACADSHARP_BASELINE"),
        "the failure does not name ACADSHARP_BASELINE:\n{all}"
    );
}
