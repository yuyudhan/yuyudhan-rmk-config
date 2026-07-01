## 1. Vendor RMK

- [x] 1.1 Clone RMK at the pinned rev: `git clone https://github.com/HaoboGu/rmk vendor/rmk && git -C vendor/rmk checkout b98204928e9a8532064ce99add1f6c4c554e08c9`
- [x] 1.2 Remove `.git` to prevent a nested repository: `rm -rf vendor/rmk/.git`
- [x] 1.3 Commit the `vendor/rmk/` tree (all four crates: `rmk`, `rmk-macro`, `rmk-types`, `rmk-config`) — commit `4e24fee`; also added `YUYUDHAN_PATCH.md` + `.patch`

## 2. Patch `Matrix` struct and run loop (`vendor/rmk/rmk/src/matrix.rs`)

- [x] 2.1 Change `use embassy_time::Timer;` (L4) to `use embassy_time::{Duration, Timer};`
- [x] 2.2 Add field `hold_back: Duration,` to the `Matrix` struct body (after `rescan_needed`), with a doc comment explaining it is non-zero only on the split central
- [x] 2.3 In `Matrix::new`, initialise the new field: `hold_back: Duration::from_ticks(0),`
- [x] 2.4 Add builder method `with_hold_back_micros(mut self, us: u64) -> Self` in the same impl block as `new`, setting `self.hold_back = Duration::from_micros(us); self`
- [x] 2.5 In `Runnable::run`, add `if self.hold_back.as_ticks() > 0 { Timer::after(self.hold_back).await; }` between `read_event()` and `publish_event_async()`

## 3. Patch central codegen (`vendor/rmk/rmk-macro/src/codegen/orchestrator.rs`)

- [x] 3.1 Split-central Normal matrix arm: appended `.with_hold_back_micros(3750)` to the `Matrix::new(...)` expression with the tuning comment
- [~] 3.2 DirectPin arm: **intentionally NOT patched** — it builds `DirectPinMatrix`, which has no `with_hold_back_micros` builder (only `Matrix` does); appending it would be latent breakage. This board is `matrix_type="normal"`, so the arm is never emitted. Left an explanatory comment in-code instead.
- [x] 3.3 Confirmed `vendor/rmk/rmk-macro/src/codegen/split/peripheral.rs:305` is untouched (peripheral `Matrix::new` stays at default 0)

## 4. Wire the patch into the build

- [x] 4.1 Added `[patch."https://github.com/HaoboGu/rmk"] rmk = { path = "vendor/rmk/rmk" }` to `Cargo.toml`
- [x] 4.2 Built `cargo build --release --bin central` — compiles clean; `Cargo.lock` now resolves `rmk` to the vendored path (no `source = "git+..."`)

## 5. Verify

- [x] 5.1 `just build both` — both halves compiled; UF2s archived to `firmware/2026-07-01_17-28-00/`
- [~] 5.2 `just expand left … grep with_hold_back_micros` — **not run**: `cargo-expand` needs a nightly toolchain (not installed; stable-only setup). Replaced by by-construction proof: the builder call exists only in the `rmk_central` codegen path (`orchestrator.rs`), reached only by `src/central.rs`; both binaries compile.
- [~] 5.3 `just expand right …` — same as 5.2; peripheral matrix is built in a separate unpatched file (`split/peripheral.rs`), reached only by `src/peripheral.rs`.
- [ ] 5.4 Flash both halves (`just flash left`, then `just flash right`) and type fast cross-hand rolls for ~1 min; confirm no transposed characters — **HARDWARE, yours to run**
- [ ] 5.5 If residual flips persist: raise hold-back in `orchestrator.rs` (6000→8000 µs) **only while `for`-type same-hand-ahead flips stay absent**; rebuild, reflash, retest — **yours to run**
