#!/usr/bin/env bash
# Merge the latest upstream WordCraft (storytold/wordcraft) into the current branch.
#
#   brand/sync-upstream.sh            merge upstream/main
#   brand/sync-upstream.sh <ref>      merge a tag or commit, e.g. v0.11.0
#
# Resolves the conflicts the fork expects on its own (upstream's CI workflows, which the fork
# removed), stops on any other conflict, then runs `cargo xtask brand` to list new upstream
# strings that still show the old name.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
UPSTREAM_URL="https://github.com/storytold/wordcraft.git"
REF="${1:-upstream/main}"

if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "error: commit or stash your changes first" >&2
  exit 1
fi
if ! git remote get-url upstream >/dev/null 2>&1; then
  git remote add upstream "$UPSTREAM_URL"
fi
git fetch upstream --tags

BEFORE="$(git rev-parse HEAD)"
if git merge --no-edit "$REF" -m "Merge upstream WordCraft $REF"; then
  echo "==> merged cleanly"
else
  # Upstream's workflows were deleted in the fork; when upstream changes one, keep it deleted.
  git diff --name-only --diff-filter=U | while read -r f; do
    case "$f" in
      .github/workflows/fork.yml) ;;
      .github/workflows/*) git rm -q -- "$f" && echo "    kept deleted: $f" ;;
    esac
  done
  if [ -n "$(git diff --name-only --diff-filter=U)" ]; then
    echo "error: conflicts left to resolve by hand:" >&2
    git diff --name-only --diff-filter=U >&2
    echo "Resolve them, then: git commit && cargo xtask brand" >&2
    exit 1
  fi
  git commit --no-edit
  echo "==> merged (workflow conflicts resolved automatically)"
fi

# Upstream's new workflows arrive as new files: remove them too.
for f in .github/workflows/*; do
  [ "$f" = .github/workflows/fork.yml ] && continue
  [ -e "$f" ] || continue
  git rm -q -- "$f" && echo "    removed new upstream workflow: $f"
done
if [ -n "$(git diff --cached --name-only)" ]; then
  git commit -q -m "Fork: drop upstream workflows"
fi

echo "==> upstream commits merged: $(git rev-list --count "$BEFORE..HEAD")"
echo "==> checking for upstream brand strings"
cargo xtask brand
