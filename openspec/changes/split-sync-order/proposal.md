## Why

On the wireless BLE split, keys pressed near-simultaneously on opposite halves occasionally arrive at the host out of order — a left-then-right roll can appear right-then-left. This is a known BLE asymmetry: the central half processes its own keypresses near-instantly while peripheral keypresses travel an extra BLE connection interval (~3.75 ms avg, 7.5 ms worst), causing the FIFO event channel to receive them out of sequence.

## What Changes

- Add a `hold_back: Duration` field to RMK's `Matrix` struct (default 0 — no behavioural change for existing users or the peripheral binary).
- Add a builder `with_hold_back_micros(us: u64) -> Self` on `Matrix` to configure the delay.
- In `Matrix::run`, apply the hold-back before publishing each event when non-zero.
- Modify the `rmk_central` proc-macro codegen to append `.with_hold_back_micros(3750)` to the central's `Matrix::new(...)` call; the `rmk_peripheral` codegen is left unchanged (stays 0).
- Vendor the RMK workspace at the pinned revision (`b98204928e9a8532064ce99add1f6c4c554e08c9`) into `vendor/rmk/` and wire it in via `[patch]` in `Cargo.toml`.

## Capabilities

### New Capabilities

- `split-key-ordering`: Reduces cross-half key misordering in normal typing via a central-side hold-back delay that narrows the BLE latency asymmetry between halves. Occasional flips at very fast inter-hand rolls may persist; the delay is a tunable heuristic (default 3750 µs), not a guarantee.

### Modified Capabilities

_None — no existing spec-level requirements are changing._

## Impact

- **`vendor/rmk/`** (new): locally-patched copy of the upstream RMK workspace; two files edited (`rmk/src/matrix.rs`, `rmk-macro/src/codegen/orchestrator.rs`).
- **`Cargo.toml`**: new `[patch."https://github.com/HaoboGu/rmk"]` section pointing at `vendor/rmk/rmk`.
- **Firmware behaviour**: left-hand keys gain ~3.75 ms of processing latency (imperceptible in practice). Cross-half misordering is reduced for typical typing speeds; residual flips remain possible at inter-key intervals below the BLE skew window and require tuning the hold-back value upward.
- **No breaking change**: `Matrix::new` signature is unchanged; peripheral binary behaviour is unchanged; `max_latency` and connection interval stay as-is.
