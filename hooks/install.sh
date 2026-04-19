#!/bin/sh

HOOKS_DIR="$(git rev-parse --git-dir)/hooks"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Installing git hooks..."

for hook in "$SCRIPT_DIR"/*; do
    name="$(basename "$hook")"
    [ "$name" = "install.sh" ] && continue
    cp "$hook" "$HOOKS_DIR/$name"
    chmod +x "$HOOKS_DIR/$name"
    echo "  Installed $name"
done

echo "Done."
