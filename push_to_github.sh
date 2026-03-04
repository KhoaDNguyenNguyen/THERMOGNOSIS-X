#!/usr/bin/env bash
# push_to_github.sh — THERMOGNOSIS-X Git Automation
# ===================================================
# Usage:
#   ./push_to_github.sh                  # auto-generates commit message
#   ./push_to_github.sh "custom message" # use provided message
#
# Behavior:
#   1. Stages all tracked modifications and explicitly listed new files
#   2. Commits with a timestamped message (or your custom message)
#   3. Pushes to origin/<current-branch>
#
# The script does NOT commit:
#   - data/  (large binaries, parquet, duckdb — covered by .gitignore)
#   - starrydata_mirror*/  (raw mirror data)
#   - __pycache__/, *.pyc, thermognosis.egg-info/
#   - Any file matched by .gitignore

set -euo pipefail

# ---------------------------------------------------------------------------
# Colour helpers
# ---------------------------------------------------------------------------
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------
command -v git >/dev/null 2>&1 || error "git not found in PATH"

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" \
    || error "Not inside a git repository"
cd "$REPO_ROOT"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
info "Repository: $REPO_ROOT"
info "Branch:     $BRANCH"

# ---------------------------------------------------------------------------
# Staged content: source files only (respects .gitignore automatically)
# ---------------------------------------------------------------------------
info "Staging tracked modifications ..."
git add --update   # stages modifications and deletions of already-tracked files

# Stage new source files explicitly (not caught by --update)
NEW_SOURCE_PATTERNS=(
    "rust_core/src/*.rs"
    "scripts/*.py"
    "scripts/*.sh"
    "*.sh"
    "*.toml"
    "*.cfg"
    "*.ini"
    ".gitignore"
    "CLAUDE.md"
    "README*"
    "pyproject.toml"
    "Cargo.toml"
    "Cargo.lock"
)
for pat in "${NEW_SOURCE_PATTERNS[@]}"; do
    # Use git add with force-exclude via pathspec; --ignore-errors skips unmatched globs
    git add --ignore-errors -- "$pat" 2>/dev/null || true
done

# ---------------------------------------------------------------------------
# Check there is something to commit
# ---------------------------------------------------------------------------
if git diff --cached --quiet; then
    warn "Nothing staged — working tree is clean. Nothing to commit."
    exit 0
fi

# ---------------------------------------------------------------------------
# Commit message
# ---------------------------------------------------------------------------
if [[ $# -ge 1 && -n "$1" ]]; then
    MSG="$1"
else
    TIMESTAMP="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    CHANGED_COUNT="$(git diff --cached --name-only | wc -l | tr -d ' ')"
    MSG="chore: auto-commit ${CHANGED_COUNT} file(s) — ${TIMESTAMP}"
fi

info "Committing: \"$MSG\""
git commit -m "$MSG"

# ---------------------------------------------------------------------------
# Push
# ---------------------------------------------------------------------------
info "Pushing to origin/$BRANCH ..."

# Check if upstream tracking branch exists
if git rev-parse --abbrev-ref --symbolic-full-name "@{u}" >/dev/null 2>&1; then
    git push
else
    warn "No upstream tracking branch found — pushing with -u"
    git push -u origin "$BRANCH"
fi

info "Done. Pushed to origin/$BRANCH"
