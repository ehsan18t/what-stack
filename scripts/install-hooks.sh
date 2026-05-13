#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/hooks"
GIT_HOOKS_DIR="$(git -C "$REPO_ROOT" rev-parse --path-format=absolute --git-path hooks)"

mkdir -p "$GIT_HOOKS_DIR"

for hook in pre-commit pre-push commit-msg; do
    cp "$HOOKS_DIR/$hook" "$GIT_HOOKS_DIR/$hook"
    chmod +x "$GIT_HOOKS_DIR/$hook"
    echo "  $hook: installed"
done

echo ""
echo "Git hooks installed."
echo "Install lint targets: rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc"
