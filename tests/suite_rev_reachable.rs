//! `SUITE_REV` has to name a commit the suite's `main` reaches.
//!
//! `tests/suite_pin_check.rs` proves the pin's shape. This proves the pin is
//! real: it clones acadsharp-rs-tests' `main` without blobs, fetches the pin into
//! the clone and asks `tools/suite_pin_check.sh reachable` whether `main` reaches
//! it. The ancestry rule lives in that script once, so this test and the job
//! cannot disagree. The case it exists for is a rewritten history, where the old
//! commit stays fetchable by sha while no branch reaches it.
//!
//! This needs the network. Without it the test fails and says so, unless
//! `ACADSHARP_OFFLINE=1`, which skips it out loud.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn pin() -> String {
    std::fs::read_to_string(root().join("SUITE_REV"))
        .unwrap()
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap()
        .to_string()
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn suite_rev_is_on_the_suite_main_branch() {
    if std::env::var("ACADSHARP_OFFLINE").as_deref() == Ok("1") {
        eprintln!("ACADSHARP_OFFLINE=1, so I did NOT check that SUITE_REV is on the suite's main");
        return;
    }
    let url = std::env::var("ACADSHARP_SUITE_URL")
        .unwrap_or_else(|_| "https://github.com/libviprs/acadsharp-rs-tests.git".to_string());
    let sha = pin();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("suite_rev_reachable");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["remote", "add", "origin", &url]);
    // `main` on purpose: only an ancestor passes, so an open suite PR head cannot
    // hide here the way it may on a PR ref.
    git(
        &dir,
        &["fetch", "--quiet", "--filter=blob:none", "origin", "+refs/heads/main:refs/remotes/origin/main", &sha],
    );
    let out = Command::new(root().join("tools").join("suite_pin_check.sh"))
        .args(["reachable"])
        .arg(dir.join(".git"))
        .arg(&sha)
        .env("GITHUB_REF_NAME", "main")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "SUITE_REV {sha} is not on {url}'s main. GitHub may still serve it by sha, which is why the Suite job's fetch step can pass, but no branch reaches it:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
