#!/usr/bin/env bash
# Fetch the ACadSharp conformance corpus at the pinned commit.
#
# The corpus is the DWG fixtures and, more importantly, the canonical record
# dump ACadSharp produced for each one. Those dumps are the oracle: they were
# recorded by running the real ACadSharp through the VIPRS flattener, which is
# not something this repository can do, since it has no .NET and is not getting
# one.
#
#   scripts/fetch-acadsharp-corpus.sh [dest]
#
# Then run the comparison:
#
#   ACADSHARP_CORPUS=<dest> cargo test --test acadsharp_conformance
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
pin="$here/../tests/acadsharp/CORPUS.pin"
dest="${1:-${ACADSHARP_CORPUS:-$here/../target/acadsharp-corpus}}"

repo=$(awk -F'= *' '/^repo/{print $2}' "$pin" | tr -d ' ')
commit=$(awk -F'= *' '/^commit/{print $2}' "$pin" | tr -d ' ')
sub=$(awk -F'= *' '/^path/{print $2}' "$pin" | tr -d ' ')

if [ -d "$dest/expectations" ]; then
  echo "corpus already at $dest"
  exit 0
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

case "$commit" in
  *[!0-9a-f]* | "") bad=1 ;;
  *) [ "${#commit}" -eq 40 ] && bad=0 || bad=1 ;;
esac
if [ "$bad" -eq 1 ]; then
  echo "error: $pin names '$commit', which is not a full 40-character commit sha." >&2
  echo "       A branch name is a valid refspec and would let the oracle float." >&2
  exit 1
fi

echo "==> cloning $repo at $commit"
git -C "$tmp" init -q .
git -C "$tmp" remote add origin "$repo"

# The pin has to be reachable from the source repository's main. A fetch by sha
# also works for a commit nothing references any more, until GitHub collects it,
# so a successful fetch proves nothing about that. Fetch main without blobs
# (history and trees only) and ask. Nothing is copied to $dest before this.
git -C "$tmp" fetch -q --filter=blob:none --no-tags origin \
  "+refs/heads/main:refs/remotes/origin/main"
if ! git -C "$tmp" merge-base --is-ancestor "$commit" refs/remotes/origin/main 2>/dev/null; then
  twin="no commit on main"
  if git -C "$tmp" fetch -q --filter=blob:none --no-tags origin "$commit" 2>/dev/null; then
    tree=$(git -C "$tmp" rev-parse "$commit^{tree}")
    twin=$(git -C "$tmp" log --format='%H %T' refs/remotes/origin/main \
      | awk -v t="$tree" '$2 == t && !f {print $1; f = 1}')
    [ -n "$twin" ] || twin="no commit on main"
  fi
  echo "error: $pin names $commit, which is not on $repo's main." >&2
  echo "       GitHub may serve it by sha today, but no branch reaches it and it goes away" >&2
  echo "       whenever unreferenced objects are collected." >&2
  echo "       The commit on main with the same tree is: $twin" >&2
  exit 1
fi

# Only the corpus directory's blobs are wanted, not the whole repository's.
git -C "$tmp" sparse-checkout set --no-cone "/$sub/"
git -C "$tmp" checkout -q --detach "$commit"
got=$(git -C "$tmp" rev-parse HEAD)
[ "$got" = "$commit" ] || {
  echo "error: checked out $got, want $commit" >&2
  exit 1
}

mkdir -p "$(dirname "$dest")"
rm -rf "$dest"
cp -R "$tmp/$sub" "$dest"
echo "==> corpus at $dest"
echo "    fixtures:     $(find "$dest/fixtures" -name '*.dwg' | wc -l | tr -d ' ')"
echo "    expectations: $(find "$dest/expectations" -name '*.txt' | wc -l | tr -d ' ')"
