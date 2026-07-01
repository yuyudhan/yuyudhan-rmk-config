# Vendored RMK — yuyudhan-1 local patch

This directory is a **committed, pinned copy** of upstream RMK
(`https://github.com/HaoboGu/rmk`) at revision:

    b98204928e9a8532064ce99add1f6c4c554e08c9

It is wired into the build by the `[patch."https://github.com/HaoboGu/rmk"]`
section in the repo-root `Cargo.toml`, which points `rmk` at `vendor/rmk/rmk`.
Patching `rmk` alone cascades to the sibling crates (`rmk-macro`, `rmk-types`,
`rmk-config`) via RMK's own `../` path-dependencies — no separate patch entries.

We vendor (rather than fork on GitHub) so the whole toolchain builds offline from
a fresh `git clone` with zero external dependencies, and so this exact source is
under our control.

## What we changed vs upstream

Two files, captured in `YUYUDHAN_PATCH.patch` (apply with `patch -p1` from this
directory). The change adds a **central-side hold-back delay** that fixes cross-half
key-order garble on the BLE split (left↔right transpositions like `for`→`ofr`,
`like`→`liek`). Full rationale: `openspec/changes/split-sync-order/`.

1. `rmk/src/matrix.rs`
   - Import `Duration` alongside `Timer`.
   - Add a `hold_back: Duration` field to `Matrix` (default `0` = no effect).
   - Initialise it to `0` in `Matrix::new` (signature unchanged — all existing
     callers, including the peripheral, keep working with no delay).
   - Add builder `with_hold_back_micros(us: u64) -> Self`.
   - In `Matrix::run`, sleep `hold_back` before publishing each event when non-zero.

2. `rmk-macro/src/codegen/orchestrator.rs`
   - The `rmk_central` codegen appends `.with_hold_back_micros(3750)` to the split
     central's `Matrix::new(...)` (Normal-matrix arm only).
   - The DirectPin arm is intentionally left unpatched (`DirectPinMatrix` has no such
     builder; this board is `matrix_type = "normal"`). A comment records why.
   - The peripheral codegen (`codegen/split/peripheral.rs`) is untouched, so the
     right half keeps `hold_back = 0`.

Gating is **by construction**, not by cargo feature — so `just check` and `just build`
compile the same central binary (a feature gate would diverge them, since
`just check` builds `central` with no features).

## Tuning the hold-back

The value lives in ONE place: the `.with_hold_back_micros(3750)` call in
`rmk-macro/src/codegen/orchestrator.rs` (central Normal arm). It is **directional**:

- Raising it fixes *peripheral-late* flips (right key lands behind an earlier left
  key, e.g. `like`→`liek`).
- But it makes *peripheral-ahead* flips (right key lands ahead of an earlier left
  key, e.g. `for`→`ofr`) MORE likely.

So tune on evidence, not comfort:

1. Start at **3750 µs** (current).
2. If `liek`-type flips persist AND `for` stays clean → raise to **6000**, then **8000**.
3. The ceiling is **not** the ~12000 µs comfort limit — it is "`ofr`-type flips
   reappear". Back off the moment they do.

After any change: `just build both`, reflash BOTH halves, and re-run the hardware
typing test (fast cross-hand rolls for ~1 min).

## Updating to a newer upstream RMK (deliberate, test-gated — never automatic)

Do NOT auto-pull. The pin (`b982049`) is load-bearing: newer RMK can break the
per-profile `enable_flow_tap` this keymap relies on, and the two edits above are
anchored to this revision's code shape (they will move if upstream refactors).

To update:

1. Pick a specific new upstream rev (read its CHANGELOG; verify per-profile
   `enable_flow_tap` still exists).
2. Re-vendor:
   ```
   rm -rf vendor/rmk
   git clone https://github.com/HaoboGu/rmk vendor/rmk
   git -C vendor/rmk checkout <NEW_REV>
   rm -rf vendor/rmk/.git
   ```
3. Re-apply the patch:
   ```
   cd vendor/rmk && patch -p1 < YUYUDHAN_PATCH.patch
   ```
   If a hunk fails (upstream moved the code), apply it by hand to the new locations,
   then regenerate the patch:
   ```
   # after fixing the two files by hand, from repo root, against a pristine checkout
   diff -u <pristine>/rmk/src/matrix.rs vendor/rmk/rmk/src/matrix.rs
   diff -u <pristine>/rmk-macro/src/codegen/orchestrator.rs vendor/rmk/rmk-macro/src/codegen/orchestrator.rs
   ```
   (paths rewritten to `a/…` `b/…`), and overwrite `YUYUDHAN_PATCH.patch`.
4. Update the rev string in the root `Cargo.toml` `rmk` git line and in this file.
5. `just build both`.
6. **MANDATORY**: reflash both halves and run the hardware typing test before
   trusting the update. A clean build is NOT sufficient — the fix is a BLE timing
   property only observable on hardware.
