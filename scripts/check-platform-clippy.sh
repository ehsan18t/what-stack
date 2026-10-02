#!/bin/sh
# what-stack - Cross-target Clippy gate.

set -eu

TARGETS="x86_64-unknown-linux-gnu x86_64-pc-windows-msvc"

require_command() {
    command_name="$1"

    if command -v "$command_name" >/dev/null 2>&1; then
        return 0
    fi

    echo ""
    echo "ERROR: REQUIRED COMMAND NOT FOUND"
    echo "  '$command_name' is required to run the cross-target Clippy gate."
    exit 1
}

require_command cargo
require_command rustc
require_command rustup

installed_targets="$(rustup target list --installed)"
missing_targets=""
host_target="$(rustc -vV | sed -n 's/^host: //p')"
host_target_supported="false"

for target in $TARGETS; do
    if ! printf '%s\n' "$installed_targets" | grep -Fx "$target" >/dev/null 2>&1; then
        missing_targets="$missing_targets $target"
    fi

    if [ "$target" = "$host_target" ]; then
        host_target_supported="true"
    fi
done

if [ -n "$missing_targets" ]; then
    echo ""
    echo "ERROR: MISSING RUST TARGETS"
    echo "  Install the supported lint targets first:"
    echo "    rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc"
    echo ""
    echo "  Missing targets:"
    for target in $missing_targets; do
        echo "    - $target"
    done
    exit 1
fi

if [ "$host_target_supported" = "true" ]; then
    echo "Detected supported host target: $host_target"
else
    echo "Host target '$host_target' is not one of the supported release targets."
fi

for target in $TARGETS; do
    if [ "$target" = "$host_target" ]; then
        echo "-> Running native clippy for $target (all-targets)..."
        cargo clippy --locked --all-targets --target "$target" -- -D warnings
    else
        echo "-> Running cross-target clippy for $target (lib only)..."
        cargo clippy --locked --lib --target "$target" -- -D warnings
    fi

    echo "  OK $target"
done
