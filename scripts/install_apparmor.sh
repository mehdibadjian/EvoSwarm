#!/bin/sh
set -e

# Installer script for EvoSwarm bubblewrap AppArmor profile (e0-3).
# Installs /etc/apparmor.d/bwrap (or specified --dest-dir) and reloads via apparmor_parser.

DEST_DIR="/etc/apparmor.d"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --dest-dir)
            if [ -z "$2" ]; then
                echo "Error: --dest-dir requires an argument" >&2
                exit 1
            fi
            DEST_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--dest-dir <DIR>]"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROFILE_SRC="$REPO_ROOT/packaging/apparmor/bwrap"

if [ ! -f "$PROFILE_SRC" ]; then
    echo "Error: Profile source not found at $PROFILE_SRC" >&2
    exit 1
fi

# Ensure destination directory exists and is writable
if [ ! -d "$DEST_DIR" ]; then
    mkdir -p "$DEST_DIR" 2>/dev/null || {
        echo "Error: Failed to create or access destination directory: $DEST_DIR" >&2
        exit 1
    }
fi

DEST_FILE="$DEST_DIR/bwrap"

# Test writability
if ! touch "$DEST_FILE" 2>/dev/null; then
    echo "Error: Permission denied writing to $DEST_FILE. Run as root or with sudo." >&2
    exit 1
fi

# Copy profile
cp "$PROFILE_SRC" "$DEST_FILE"
chmod 644 "$DEST_FILE"

# If apparmor_parser exists and running as root, reload profile
if command -v apparmor_parser >/dev/null 2>&1; then
    if [ "$(id -u)" -eq 0 ]; then
        apparmor_parser -r -W "$DEST_FILE" 2>/dev/null || true
    fi
fi

echo "Successfully installed AppArmor profile to $DEST_FILE"
exit 0
