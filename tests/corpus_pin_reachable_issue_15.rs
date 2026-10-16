//! Issue #15: `tests/acadsharp/CORPUS.pin` has to name a libviprs-dep commit
//! that `main` reaches, and the fetch script has to refuse one it does not.
//!
//! libviprs-dep's `main` was rewritten with byte-identical trees after the pin
//! was written. The pinned commit is still fetchable by sha, because GitHub
//! keeps unreferenced objects until it collects them, so
//! `scripts/fetch-acadsharp-corpus.sh` works, the conformance job works, and
//! both stop working on a day nobody picks.
//!
//! Four tests. Two are red until the fix: the committed pin is on
//! libviprs-dep's `main`, and the fetch script refuses a pin `main` does not
//! reach. A third is red for acadrust#11 (b), which this issue absorbs: the
//! script has to assert after the checkout that `git rev-parse HEAD` is the
//! pinned sha, so a pin holding a branch name is refused instead of floating.
//! The fourth is the positive control: the script still fetches a pin `main`
//! does reach, and lays down the same 52 fixtures and 45 record dumps.
//!
//! The three that talk to libviprs-dep need the network. Without it they fail
//! and say so, unless `ACADRUST_OFFLINE=1`, which skips them out loud. The
//! branch-name one runs against a local repository and needs nothing.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The libviprs-dep commit the rewrite orphaned, which `CORPUS.pin` names
/// today. Same tree as `bc351a9` on `main`.
const ORPHANED: &str = "0d475bf309af883db621ef159af150be1087a222";

/// The commit on libviprs-dep's `main` with the same tree as the orphaned one,
/// which is where the pin should move. The positive control pins it.
const ON_MAIN: &str = "bc351a906d9085f256fb2226fd6050f72e2d3325";

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

/// The script reads the pin from its own tree, so these tests give it a copy
/// of that tree with a pin of their choosing. That keeps them about the
/// mechanism after `CORPUS.pin` itself moves. Returns the copied tree.
fn tree_with_pin(dir: &Path, repo: &str, commit: &str) -> PathBuf {
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
            "# a pin written by tests/corpus_pin_reachable_issue_15.rs\n\
             repo   = {repo}\ncommit = {commit}\npath   = acadsharp/tests\n"
        ),
    )
    .unwrap();
    tree
}

/// Runs the copied script into `dest`, with nothing in the environment
/// steering where the corpus goes.
fn fetch_corpus(tree: &Path, dest: &Path) -> Output {
    Command::new("bash")
        .arg(tree.join("scripts/fetch-acadsharp-corpus.sh"))
        .arg(dest)
        .env_remove("ACADSHARP_CORPUS")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run scripts/fetch-acadsharp-corpus.sh")
}

/// How many files under `dir`, at any depth, carry the extension.
fn files_with(dir: &Path, ext: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| {
            let path = e.path();
            if path.is_dir() {
                files_with(&path, ext)
            } else {
                usize::from(path.extension().is_some_and(|x| x == ext))
            }
        })
        .sum()
}

/// A stand-in for libviprs-dep: one commit on `main` holding a corpus of one
/// fixture and one record dump under `acadsharp/tests`, the path the pin
/// names. Returns the sha of that commit.
fn upstream_fixture(dir: &Path) -> String {
    std::fs::create_dir_all(dir.join("acadsharp/tests/fixtures")).unwrap();
    std::fs::create_dir_all(dir.join("acadsharp/tests/expectations")).unwrap();
    std::fs::write(dir.join("acadsharp/tests/fixtures/a.dwg"), b"not a dwg\n").unwrap();
    std::fs::write(
        dir.join("acadsharp/tests/expectations/a.txt"),
        "a record dump\n",
    )
    .unwrap();
    let steps: [&[&str]; 3] = [
        &["init", "-q", "-b", "main"],
        &["add", "."],
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-q",
            "-m",
            "corpus",
        ],
    ];
    for args in steps {
        let out = git(dir, args);
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            joined(&out)
        );
    }
    joined(&git(dir, &["rev-parse", "HEAD"]))
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
    if git(
        &repo,
        &[
            "merge-base",
            "--is-ancestor",
            &pin,
            "refs/remotes/origin/main",
        ],
    )
    .status
    .success()
    {
        return;
    }
    let main = joined(&git(&repo, &["rev-parse", "refs/remotes/origin/main"]));
    let tree = joined(&git(&repo, &["rev-parse", &format!("{pin}^{{tree}}")]));
    let log = joined(&git(
        &repo,
        &["log", "--format=%H %T", "refs/remotes/origin/main"],
    ));
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
    // A copy of the tree with a pin holding the orphaned commit.
    let dir = scratch("issue_15_fetch");
    let tree = tree_with_pin(&dir, &pin_field("repo"), ORPHANED);
    let dest = dir.join("corpus");
    let out = fetch_corpus(&tree, &dest);
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

#[test]
fn issue_15_the_fetch_script_still_fetches_a_pin_main_reaches() {
    if offline() {
        eprintln!("issue_15: ACADRUST_OFFLINE=1, so I did NOT check that the fetch script still fetches a reachable pin");
        return;
    }
    // The positive control. bc351a9 is on libviprs-dep's main (behind its
    // head, which is what every pin becomes the moment main moves on) and it
    // carries the same tree as the orphaned pin, so the corpus the script lays
    // down is byte for byte the one the conformance job reads today. A
    // reachability check that refused everything would fail here.
    let dir = scratch("issue_15_fetch_reachable");
    let tree = tree_with_pin(&dir, &pin_field("repo"), ON_MAIN);
    let dest = dir.join("corpus");
    let out = fetch_corpus(&tree, &dest);
    let all = joined(&out);
    assert_eq!(
        out.status.code(),
        Some(0),
        "fetch-acadsharp-corpus.sh refused {ON_MAIN}, which is on libviprs-dep's main (expected \
         exit 0). Either main was rewritten again under this control, or the script refuses more \
         than it should. This check needs the network; set ACADRUST_OFFLINE=1 to skip it on \
         purpose. It said:\n{all}"
    );
    assert!(
        dest.join("expectations").is_dir(),
        "the script said yes and left no corpus at {}:\n{all}",
        dest.display()
    );
    assert_eq!(
        (
            files_with(&dest.join("fixtures"), "dwg"),
            files_with(&dest.join("expectations"), "txt"),
        ),
        (52, 45),
        "the corpus at {} is not the 52 fixtures and 45 record dumps the tree at {ON_MAIN} \
         holds, so the script fetched something else:\n{all}",
        dest.display()
    );
}

#[test]
fn issue_15_the_fetch_script_refuses_a_pin_that_names_a_branch() {
    // acadrust#11 (b), which this issue absorbs. `commit` is used as a fetch
    // refspec and nothing checks what came back, so a pin holding a branch
    // name is a valid refspec, fetches fine, and floats with the branch while
    // the cache key still hashes a file that looks pinned. After the checkout
    // the script has to assert that `git rev-parse HEAD` is the pinned sha,
    // and a branch name never is. A local repository stands in for
    // libviprs-dep, so this one needs no network.
    let dir = scratch("issue_15_fetch_branch_name");
    let upstream = dir.join("upstream");
    let head = upstream_fixture(&upstream);
    let tree = tree_with_pin(&dir, &format!("file://{}", upstream.display()), "main");
    let dest = dir.join("corpus");
    let out = fetch_corpus(&tree, &dest);
    let all = joined(&out);
    assert_ne!(
        out.status.code(),
        Some(0),
        "fetch-acadsharp-corpus.sh fetched a pin whose commit is the branch name `main` (today \
         {head}) and exited 0. A branch name is a valid refspec, so the fetch works and the \
         oracle floats with the branch; after the checkout the script has to assert that \
         `git rev-parse HEAD` is the pinned sha, and `main` is not one. It said:\n{all}"
    );
    assert!(
        !dest.join("expectations").is_dir(),
        "the script refused, but left a corpus at {} anyway",
        dest.display()
    );
}
