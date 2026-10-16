//! Issue #15: `tests/acadsharp/CORPUS.pin` has to name a libviprs-dep commit
//! that `main` reaches, and the fetch script has to refuse one it does not.
//!
//! libviprs-dep's `main` was rewritten with byte-identical trees after the pin
//! was written. The pinned commit is still fetchable by sha, because GitHub
//! keeps unreferenced objects until it collects them, so
//! `scripts/fetch-acadsharp-corpus.sh` works, the conformance job works, and
//! both stop working on a day nobody picks.
//!
//! Both tests need the network. Without it they fail and say so, unless
//! `ACADRUST_OFFLINE=1`, which skips them out loud.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The libviprs-dep commit the rewrite orphaned, which `CORPUS.pin` names
/// today. Same tree as `bc351a9` on `main`.
const ORPHANED: &str = "0d475bf309af883db621ef159af150be1087a222";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn offline() -> bool {
    std::env::var("ACADRUST_OFFLINE").as_deref() == Ok("1")
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// `repo` and `commit` out of `CORPUS.pin`, read the way the fetch script's
/// awk reads them.
fn pin_field(field: &str) -> String {
    let path = repo_root().join("tests/acadsharp/CORPUS.pin");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == field).then(|| v.trim().to_string())
        })
        .unwrap_or_else(|| panic!("CORPUS.pin has no `{field} =` line"))
}

fn joined(out: &Output) -> String {
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s.trim().to_string()
}

fn git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap_or_else(|e| panic!("could not run git {args:?}: {e}"))
}

#[test]
fn issue_15_corpus_pin_is_on_libviprs_dep_main() {
    if offline() {
        eprintln!("issue_15: ACADRUST_OFFLINE=1, so I did NOT check that CORPUS.pin is on libviprs-dep's main");
        return;
    }
    let url = pin_field("repo");
    let pin = pin_field("commit");
    let repo = scratch("issue_15_dep_main").join("libviprs-dep");
    let out = Command::new("git")
        .args([
            "clone",
            "--quiet",
            "--filter=blob:none",
            "--no-checkout",
            "--single-branch",
            "--branch",
            "main",
        ])
        .arg(&url)
        .arg(&repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run git clone");
    assert!(
        out.status.success(),
        "could not clone {url}'s main: {}\nThis check needs the network. Set ACADRUST_OFFLINE=1 to skip it on purpose.",
        joined(&out)
    );
    let fetched = git(&repo, &["fetch", "--quiet", "origin", &pin]);
    assert!(
        fetched.status.success(),
        "CORPUS.pin's commit {pin} cannot be fetched from {url} at all: {}",
        joined(&fetched)
    );
    if git(&repo, &["merge-base", "--is-ancestor", &pin, "refs/remotes/origin/main"])
        .status
        .success()
    {
        return;
    }
    let main = joined(&git(&repo, &["rev-parse", "refs/remotes/origin/main"]));
    let tree = joined(&git(&repo, &["rev-parse", &format!("{pin}^{{tree}}")]));
    let log = joined(&git(&repo, &["log", "--format=%H %T", "refs/remotes/origin/main"]));
    let twin = log
        .lines()
        .find_map(|l| {
            let (c, t) = l.split_once(' ')?;
            (t == tree).then(|| c.to_string())
        })
        .unwrap_or_else(|| "no commit on main".to_string());
    panic!(
        "CORPUS.pin names {pin}, which is not on libviprs-dep's main (main is at {main}). \
         GitHub still serves it by sha, so the corpus fetch works, but no branch reaches it and it \
         disappears whenever GitHub collects unreferenced objects. \
         The commit on main with the same tree ({tree}) is: {twin}."
    );
}

#[test]
fn issue_15_the_fetch_script_refuses_a_pin_main_does_not_reach() {
    if offline() {
        eprintln!("issue_15: ACADRUST_OFFLINE=1, so I did NOT check that the fetch script refuses an unreachable pin");
        return;
    }
    // The script reads the pin from its own tree, so give it a copy of that
    // tree with a pin holding the orphaned commit. That keeps this test about
    // the mechanism after CORPUS.pin itself moves.
    let dir = scratch("issue_15_fetch");
    let tree = dir.join("tree");
    std::fs::create_dir_all(tree.join("scripts")).unwrap();
    std::fs::create_dir_all(tree.join("tests/acadsharp")).unwrap();
    std::fs::copy(
        repo_root().join("scripts/fetch-acadsharp-corpus.sh"),
        tree.join("scripts/fetch-acadsharp-corpus.sh"),
    )
    .unwrap();
    std::fs::write(
        tree.join("tests/acadsharp/CORPUS.pin"),
        format!(
            "# the libviprs-dep commit the rewrite of main orphaned\n\
             repo   = {}\ncommit = {ORPHANED}\npath   = acadsharp/tests\n",
            pin_field("repo")
        ),
    )
    .unwrap();
    let dest = dir.join("corpus");
    let out = Command::new("bash")
        .arg(tree.join("scripts/fetch-acadsharp-corpus.sh"))
        .arg(&dest)
        .env_remove("ACADSHARP_CORPUS")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run scripts/fetch-acadsharp-corpus.sh");
    let all = joined(&out);
    assert_ne!(
        out.status.code(),
        Some(0),
        "fetch-acadsharp-corpus.sh fetched the corpus at {ORPHANED}, which libviprs-dep's main does \
         not reach, and exited 0. A fetch by sha succeeds for an unreferenced commit until GitHub \
         collects it, so the fetch is not a reachability check. It said:\n{all}"
    );
    assert!(
        !dest.join("expectations").is_dir(),
        "the script refused, but left a corpus at {} anyway",
        dest.display()
    );
}
