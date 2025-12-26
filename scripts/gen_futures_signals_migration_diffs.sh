#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Generate per-file diffs for the futures-signals migration.

This compares:
  <base>:src/<file>.rs  ->  (HEAD or working tree):src/futures_signals/<file>.rs

Defaults:
  base = origin/main
  right = worktree
  out  = diff_reports/origin_main_src_vs_worktree_futures_signals

Usage:
  scripts/gen_futures_signals_migration_diffs.sh [--base <ref>] [--right head|worktree] [--out <dir>] [--no-fetch]

Examples:
  scripts/gen_futures_signals_migration_diffs.sh
  scripts/gen_futures_signals_migration_diffs.sh --base origin/bevy_0.17
  scripts/gen_futures_signals_migration_diffs.sh --right head
  scripts/gen_futures_signals_migration_diffs.sh --out diff_reports/custom_run
EOF
}

base="origin/main"
right="worktree" # 'worktree' includes unstaged changes; 'head' uses HEAD:...
outdir="diff_reports/origin_main_src_vs_worktree_futures_signals"
do_fetch=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --base)
      base="${2:?missing value for --base}"
      shift 2
      ;;
    --right)
      right="${2:?missing value for --right}"
      if [[ "$right" != "worktree" && "$right" != "head" ]]; then
        echo "--right must be 'worktree' or 'head' (got: $right)" >&2
        exit 2
      fi
      shift 2
      ;;
    --out)
      outdir="${2:?missing value for --out}"
      shift 2
      ;;
    --no-fetch)
      do_fetch=0
      shift
      ;;
    *)
      echo "Unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

if [[ "$do_fetch" -eq 1 ]]; then
  git fetch origin --prune >/dev/null
fi

mkdir -p "$outdir"

files=(
  align
  column
  derive
  el
  element
  global_event_aware
  grid
  mouse_wheel_scrollable
  pointer_event_aware
  row
  stack
  text_input
  utils
  viewport_mutable
  raw
  node_builder
)

{
  echo "# Diff: ${base}:src/<file>.rs -> ${right}:src/futures_signals/<file>.rs"
  echo
  echo "Generated at: $(date -Is)"
  echo
  echo "Notes:"
  echo "- This is the migration-mapping diff (old location -> new module location)."
  echo "- src/futures_signals/mod.rs has no src/mod.rs equivalent; it is not included here."
  echo "- right=worktree includes unstaged changes; right=head compares against HEAD." 
  echo
} > "$outdir/README.md"

changed=0
tmpdir=$(mktemp -d)
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

for f in "${files[@]}"; do
  left_spec="${base}:src/${f}.rs"
  worktree_path="src/futures_signals/${f}.rs"
  out_patch="$outdir/${f}.diff"

  if [[ "$right" == "head" ]]; then
    right_spec="HEAD:${worktree_path}"
    git diff --no-color "$left_spec" "$right_spec" > "$out_patch" || true

    if git diff --quiet "$left_spec" "$right_spec"; then
      echo "- ${f}.diff OK (identical)" >> "$outdir/README.md"
      continue
    fi

    changed=$((changed+1))
    stat=$(git diff --shortstat "$left_spec" "$right_spec" | tr -d '\n')
    echo "- ${f}.diff CHANGED (${stat})" >> "$outdir/README.md"
    continue
  fi

  # right=worktree: compare base blob to current file contents (includes staged/unstaged changes).
  if [[ ! -f "$worktree_path" ]]; then
    echo "- ${f}.diff MISSING (no $worktree_path in working tree)" >> "$outdir/README.md"
    : > "$out_patch"
    continue
  fi

  left_tmp="$tmpdir/${f}.base.rs"
  git show "$left_spec" > "$left_tmp"

  # Use git diff --no-index to diff two filesystem paths. This captures unstaged changes.
  git diff --no-index --no-color -- "$left_tmp" "$worktree_path" > "$out_patch" || true

  if [[ ! -s "$out_patch" ]]; then
    echo "- ${f}.diff OK (identical)" >> "$outdir/README.md"
  else
    changed=$((changed+1))
    stat=$(git diff --no-index --shortstat -- "$left_tmp" "$worktree_path" | tr -d '\n' || true)
    echo "- ${f}.diff CHANGED (${stat})" >> "$outdir/README.md"
  fi
done

echo >> "$outdir/README.md"
echo "Changed files: ${changed}/${#files[@]}" >> "$outdir/README.md"

echo "Wrote per-file diffs to: $outdir"
