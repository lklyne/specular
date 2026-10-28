#!/usr/bin/env bash
# Reclaim disk from Rust build output across this repo's git worktrees.
#
# Each worktree that builds native/ grows its own target/ folder, often
# 10–75 GB. A shared CARGO_TARGET_DIR across worktrees was tried and gave
# wrong builds, so targets stay per-worktree and this script trims them.
#
# For every worktree it reports the target/ size, branch, and whether the
# branch's PR is merged. With --apply it then:
#   - removes worktrees whose PR is merged and that have no local changes
#   - runs `cargo sweep --time $SWEEP_DAYS` on the rest, dropping artifacts
#     that no build has used recently
# With --full, remaining targets get `cargo clean` instead of a sweep.
#
# Worktrees with a cargo/rustc process running inside them are skipped.
# The main checkout is never removed.
#
# Usage:
#   ./scripts/clean-builds.sh            # report only
#   ./scripts/clean-builds.sh --apply    # remove merged worktrees, sweep the rest
#   ./scripts/clean-builds.sh --full     # remove merged worktrees, cargo clean the rest
#
# Env knobs:
#   SWEEP_DAYS   artifact age cutoff for cargo sweep (default 2)

set -euo pipefail

mode=report
case "${1:-}" in
  --apply) mode=apply ;;
  --full) mode=full ;;
  "") ;;
  *) echo "usage: $0 [--apply|--full]" >&2; exit 2 ;;
esac
SWEEP_DAYS="${SWEEP_DAYS:-2}"

main_checkout="$(git rev-parse --path-format=absolute --git-common-dir)"
main_checkout="${main_checkout%/.git}"

busy_dirs="$(
  for pid in $(pgrep -x 'cargo|rustc' 2>/dev/null || true); do
    lsof -a -p "$pid" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p'
  done
)"

is_busy() {
  [ -n "$busy_dirs" ] && printf '%s\n' "$busy_dirs" | grep -q "^$1"
}

pr_merged() {
  local branch="$1"
  [ -n "$branch" ] || return 1
  command -v gh >/dev/null || return 1
  [ -n "$(gh pr list --head "$branch" --state merged --json number --jq '.[0].number' 2>/dev/null)" ]
}

target_dirs() {
  find "$1" -maxdepth 3 -type f -name CACHEDIR.TAG -path '*/target/*' 2>/dev/null \
    | sed 's|/CACHEDIR.TAG$||'
}

size_of() {
  du -sh "$@" 2>/dev/null | awk '{s=s" "$1} END{print s}'
}

total_before="$(df -h "$main_checkout" | awk 'NR==2{print $4}')"

git worktree list --porcelain | awk '/^worktree /{print substr($0,10)}' | while read -r wt; do
  [ -d "$wt" ] || continue
  branch="$(git -C "$wt" symbolic-ref --short -q HEAD || true)"
  targets="$(target_dirs "$wt")"
  dirty="$(git -C "$wt" status --porcelain | wc -l | tr -d ' ')"
  merged=no; pr_merged "$branch" && merged=yes

  label="${branch:-detached}"
  if [ -n "$targets" ]; then
    # shellcheck disable=SC2086
    sz="$(size_of $targets)"
  else
    sz=" -"
  fi
  printf '%-70s %-40s target:%-8s merged:%-3s dirty:%s\n' "$wt" "$label" "$sz" "$merged" "$dirty"

  [ "$mode" = report ] && continue

  if is_busy "$wt"; then
    echo "  skip: a cargo build is running here"
    continue
  fi

  if [ "$wt" != "$main_checkout" ] && [ "$merged" = yes ] && [ "$dirty" = 0 ]; then
    echo "  removing worktree (PR merged, no local changes)"
    git worktree remove "$wt"
    continue
  fi

  for t in $targets; do
    ws="$(dirname "$t")"
    if [ "$mode" = full ]; then
      echo "  cargo clean in $ws"
      (cd "$ws" && cargo clean -q)
    elif command -v cargo-sweep >/dev/null; then
      echo "  cargo sweep --time $SWEEP_DAYS in $ws"
      (cd "$ws" && cargo sweep --time "$SWEEP_DAYS" >/dev/null)
    else
      echo "  cargo-sweep not installed (cargo install cargo-sweep); skipping $ws"
    fi
  done
done

if [ "$mode" = report ]; then
  echo
  echo "Report only. Run with --apply to remove merged worktrees and sweep stale artifacts."
else
  echo
  echo "Free space: $total_before -> $(df -h "$main_checkout" | awk 'NR==2{print $4}')"
fi
