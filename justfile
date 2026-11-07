# justfile — local RMK firmware builds for the yuyudhan-1 Corne (nice!nano, BLE split).
#
# Single profile, 42-key layout. The whole keymap lives in `config/keyboard.toml`.
# Usage is `just <verb> <target>`:
#   just build left      # LEFT half  (central)  -> rmk-central.uf2
#   just build right     # RIGHT half (peripheral) -> rmk-peripheral.uf2
#   just build both      # both halves
#   just docs svg        # render config/keyboard.toml -> yuyudhan-1_keymap.svg
#   just docs html       # render config/keyboard.toml -> yuyudhan-1-viewer.html
#   just flash left      # copy newest rmk-central.uf2 onto a mounted NICENANO
#   just flash right     # copy newest rmk-peripheral.uf2 onto a mounted NICENANO
#   just flash left --clear-storage   # one-shot wipe image; then `just flash left`
#
# Every build archives its UF2(s) — a normal and a *-clear-storage.uf2 per half —
# + a config snapshot into a local firmware/<datetime>_y<ver>/ directory (gitignored)
# so you can always re-flash an older build.
#
# Run `just` (no args) to list recipes.

# Nice!nano bootloader mount point (double-tap reset to expose it).
# Override with NICENANO=/path; otherwise macOS /Volumes/NICENANO, or on Linux the
# first existing of /media/$USER/NICENANO and /run/media/$USER/NICENANO.
NICENANO := env_var_or_default("NICENANO", if os() == "macos" { "/Volumes/NICENANO" } else { if path_exists("/run/media/" + env_var_or_default("USER", "") + "/NICENANO") == "true" { "/run/media/" + env_var_or_default("USER", "") + "/NICENANO" } else { "/media/" + env_var_or_default("USER", "") + "/NICENANO" } })

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
    uf2() {
      case "{{target}}" in
        left)  cargo make uf2-central --release ;;
        right) cargo make uf2-peripheral --release ;;
        both)  cargo make uf2 --release ;;
      esac
    }
    stamp="$(date '+%Y-%m-%d_%H-%M-%S')"
    dest="firmware/${stamp}_y${version}"
    mkdir -p "$dest" build
    # Archive build/<name>.uf2 as $dest/<name><suffix>.uf2 for the halves built.
    stash() {
      case "{{target}}" in left|both)  mv build/rmk-central.uf2    "$dest/rmk-central$1.uf2";; esac
      case "{{target}}" in right|both) mv build/rmk-peripheral.uf2 "$dest/rmk-peripheral$1.uf2";; esac
    }
    # 1) Normal image from config/keyboard.toml as-is.
    uf2
    stash ""
    # 2) Clear-storage image from a temp copy with clear_storage = true, selected via
    #    KEYBOARD_TOML_PATH (an exported env var wins over .cargo/config.toml [env]).
    #    config/keyboard.toml itself is never edited.
    tmp_toml="$PWD/build/keyboard-clear-storage.toml"
    sed -E 's/^clear_storage[[:space:]]*=.*/clear_storage = true/' {{KEYMAP}} > "$tmp_toml"
    grep -qE '^clear_storage = true$' "$tmp_toml" || { echo "Could not set clear_storage in $tmp_toml" >&2; exit 1; }
    KEYBOARD_TOML_PATH="$tmp_toml" uf2
    stash "-clear-storage"
    rm -f "$tmp_toml"
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
#   just docs html   -> yuyudhan-1-viewer.html (standalone 8-layer viewer)
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
# e.g. `just flash left`                  -> newest normal build
#      `just flash left 0.0.8`            -> newest archive matching y0.0.8
#      `just flash left --clear-storage`  -> clear-storage image (wipes bonds + keymap)
#      `just flash left --dry-run`        -> only print which file would be flashed
# (flashing is destructive)
flash target *args:
    #!/usr/bin/env bash
    set -euo pipefail
    clear=0; dry=0; ver=""
    for a in {{args}}; do
      case "$a" in
        --clear-storage) clear=1 ;;
        --dry-run) dry=1 ;;
        -*) echo "Unknown flag: $a  (valid: --clear-storage --dry-run)" >&2; exit 1 ;;
        *) ver="$a" ;;
      esac
    done
    suffix=""; [ "$clear" = 1 ] && suffix="-clear-storage"
    case "{{target}}" in
      left)  name="rmk-central${suffix}.uf2" ;;
      right) name="rmk-peripheral${suffix}.uf2" ;;
      *) echo "Unknown target: {{target}}  (valid: left | right)" >&2; exit 1 ;;
    esac
    # Newest archived copy by mtime (optionally filtered by version), else the
    # loose repo-root build.
    if [ -n "$ver" ]; then
      file="$(ls -1t firmware/*_y"$ver"/"$name" 2>/dev/null | head -1 || true)"
      if [ -z "$file" ]; then
        echo "No $name found for version y$ver in firmware/." >&2; exit 1
      fi
    else
      file="$(ls -1t firmware/*/"$name" 2>/dev/null | head -1 || true)"
      [ -z "$file" ] && [ -f "$name" ] && file="$name"
      if [ -z "$file" ]; then
        echo "No $name found. Build it first: just build {{target}}" >&2; exit 1
      fi
    fi
    if [ "$clear" = 1 ]; then
      echo "!!  CLEAR-STORAGE image: wipes ALL BLE pairings and the saved (Vial) keymap"
      echo "!!  on the {{target}} half, and keeps wiping on EVERY boot until reflashed."
      echo "!!  Next: let it boot once, double-tap reset again, then run: just flash {{target}}"
    fi
    if [ "$dry" = 1 ]; then
      echo "==> [dry-run] would flash $file -> {{NICENANO}}"; exit 0
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
    if [ "$clear" = 1 ]; then
      echo "==> Storage wiped on boot. Now double-tap reset on the {{target}} half and run: just flash {{target}}"
    fi

# Remove the Rust build cache + the build/ staging dir (keeps firmware/<datetime>/ archive).
clean:
    cargo clean
    rm -rf build
