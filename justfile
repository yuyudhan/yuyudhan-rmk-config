# justfile — local RMK firmware builds for the yuyudhan-1 Corne (nice!nano, BLE split).
#
# Single profile, 40-key layout. The whole keymap lives in `config/keyboard.toml`.
# Usage is `just <verb> <target>`:
#   just build left      # LEFT half  (central)  -> rmk-central.uf2
#   just build right     # RIGHT half (peripheral) -> rmk-peripheral.uf2
#   just build both      # both halves
#   just docs svg        # render config/keyboard.toml -> yuyudhan-1_keymap.svg
#   just docs html       # render config/keyboard.toml -> yuyudhan-1-viewer.html
#   just flash left      # copy newest rmk-central.uf2 onto a mounted NICENANO
#   just flash right     # copy newest rmk-peripheral.uf2 onto a mounted NICENANO
#
# Every build archives its UF2(s) + a config snapshot into a local firmware/<datetime>/
# directory (gitignored) so you can always re-flash an older build.
#
# Run `just` (no args) to list recipes.

# Nice!nano bootloader mount point (double-tap reset to expose it).
NICENANO := "/Volumes/NICENANO"

# Keymap config (single source of truth).
KEYMAP := "config/keyboard.toml"

# List available recipes
default:
    @just --list

# One-time host toolchain setup (idempotent — safe to re-run).
setup:
    rustup target add thumbv7em-none-eabihf
    rustup component add llvm-tools
    cargo install flip-link
    cargo install cargo-make
    cargo install cargo-binutils
    cargo install cargo-hex-to-uf2

# Build firmware. target = left | right | both. Archives to firmware/<datetime>/.
build target:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
      left|right|both) ;;
      *) echo "Unknown target: {{target}}  (valid: left | right | both)" >&2; exit 1 ;;
    esac
    # Auto-bump the patch version on a full `both` build — the canonical "cut
    # firmware" path — so each release is uniquely numbered and both halves share
    # one version. `left`/`right` builds embed the current VERSION unchanged,
    # keeping the two OLEDs in sync when you flash a single half on its own.
    if [ "{{target}}" = "both" ]; then
      IFS=. read -r MAJ MIN PAT < VERSION
      PAT=$(( ${PAT:-0} + 1 ))
      printf '%s.%s.%s\n' "${MAJ:-0}" "${MIN:-0}" "$PAT" > VERSION
      echo "==> Version bumped -> y${MAJ:-0}.${MIN:-0}.${PAT} (VERSION)"
    fi
    version="$(tr -d '[:space:]' < VERSION)"
    # Compile -> hex -> uf2 via cargo-make. All outputs land in ./build/ (staging).
    case "{{target}}" in
      left)  cargo make uf2-central --release ;;
      right) cargo make uf2-peripheral --release ;;
      both)  cargo make uf2 --release ;;
    esac
    # Move the finished UF2(s) from build/ into a timestamped, version-stamped keepsake dir.
    stamp="$(date '+%Y-%m-%d_%H-%M-%S')"
    dest="firmware/${stamp}_y${version}"
    mkdir -p "$dest"
    case "{{target}}" in left|both)  [ -f build/rmk-central.uf2 ]    && mv build/rmk-central.uf2    "$dest/";; esac
    case "{{target}}" in right|both) [ -f build/rmk-peripheral.uf2 ] && mv build/rmk-peripheral.uf2 "$dest/";; esac
    # Snapshot the config + keymap visuals alongside the firmware (copies — sources stay in place).
    cp {{KEYMAP}} "$dest/"
    cp config/vial.json "$dest/" 2>/dev/null || true
    [ -f yuyudhan-1_keymap.svg ]  && cp yuyudhan-1_keymap.svg  "$dest/" || true
    [ -f yuyudhan-1-viewer.html ] && cp yuyudhan-1-viewer.html "$dest/" || true
    echo "==> Firmware y${version} archived -> $dest (build/ staging + root left clean)"
    ls -l "$dest"

# Compile-only sanity check. target = left | right | both. No .hex/.uf2 produced.
# A bad keycode/schema in config/keyboard.toml fails here with a `keyboard.toml:` panic.
check target="both":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
      left)  cargo build --release --bin central ;;
      right) cargo build --release --bin peripheral ;;
      both)  cargo build --release --bin central && cargo build --release --bin peripheral ;;
      *) echo "Unknown target: {{target}}  (valid: left | right | both)" >&2; exit 1 ;;
    esac

# Generate keymap documentation. kind = svg | html.
#   just docs svg    -> yuyudhan-1_keymap.svg  (keymap-drawer render)
#   just docs html   -> yuyudhan-1-viewer.html (standalone 7-layer viewer)
docs kind:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{kind}}" in
      svg)  python3 scripts/keymap_docs.py svg  --toml {{KEYMAP}} --out yuyudhan-1_keymap.svg ;;
      html) python3 scripts/keymap_docs.py html --toml {{KEYMAP}} --out yuyudhan-1-viewer.html ;;
      *) echo "Unknown docs kind: {{kind}}  (valid: svg | html)" >&2; exit 1 ;;
    esac

# Print the macro-expanded keymap for a half (verifies keymap compiled in).
# target = left | right. Requires cargo-expand (`cargo install cargo-expand`).
expand target="left":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
      left)  cargo expand --bin central ;;
      right) cargo expand --bin peripheral ;;
      *) echo "Unknown target: {{target}}  (valid: left | right)" >&2; exit 1 ;;
    esac

# Flash the newest built .uf2 onto a mounted NICENANO drive. target = left | right.
# Double-tap the reset button on the target half FIRST so it mounts as {{NICENANO}}.
# Pulls the newest matching .uf2 from firmware/<datetime>/ (falls back to repo root).
# e.g. `just flash left`         -> newest build
#      `just flash left 0.0.8`   -> newest archive matching y0.0.8
# (flashing is destructive)
flash target ver="":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{target}}" in
      left)  name="rmk-central.uf2" ;;
      right) name="rmk-peripheral.uf2" ;;
      *) echo "Unknown target: {{target}}  (valid: left | right)" >&2; exit 1 ;;
    esac
    # Newest archived copy by mtime (optionally filtered by version), else the
    # loose repo-root build.
    if [ -n "{{ver}}" ]; then
      file="$(ls -1t firmware/*_y{{ver}}/"$name" 2>/dev/null | head -1 || true)"
      if [ -z "$file" ]; then
        echo "No $name found for version y{{ver}} in firmware/." >&2; exit 1
      fi
    else
      file="$(ls -1t firmware/*/"$name" 2>/dev/null | head -1 || true)"
      [ -z "$file" ] && [ -f "$name" ] && file="$name"
      if [ -z "$file" ]; then
        echo "No $name found. Build it first: just build {{target}}" >&2; exit 1
      fi
    fi
    if [ ! -d "{{NICENANO}}" ]; then
      echo "{{NICENANO}} not mounted. Double-tap reset on the {{target}} half, then re-run." >&2; exit 1
    fi
    echo "==> Flashing $file -> {{NICENANO}}"
    if cp "$file" "{{NICENANO}}"/ 2>/dev/null; then
      echo "==> Done. The nice!nano reboots automatically."
    else
      echo "==> cp reported an error — normal: the nice!nano reboots and unmounts mid-write. Flash succeeded."
    fi

# Remove the Rust build cache + the build/ staging dir (keeps firmware/<datetime>/ archive).
clean:
    cargo clean
    rm -rf build
