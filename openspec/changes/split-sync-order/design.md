## Context

The yuyudhan-1 firmware is a Miryoku-style QWERTY split keyboard running RMK on two
nRF52840 (nice!nano v2) halves connected by BLE only. Both halves' key events funnel into
a single FIFO `KeyboardEvent` PubSubChannel (capacity 16) that the keyboard task drains in
pure arrival order. Central-local keys publish near-instantly; peripheral keys traverse
peripheral-scan → BLE GATT notify → central republish, adding roughly one BLE connection
interval of latency (avg 3.75 ms, worst ~7.5 ms at the 7.5 ms connection interval floor).
Because `KeyboardEvent` carries no timestamp or source tag, arrival order is the only
ordering available, so any key pressed on the right half within that latency window after a
left-half key can arrive first, inverting the intended sequence.

RMK is consumed as a pinned git dependency (`b98204928e9a8532064ce99add1f6c4c554e08c9`).
Connection parameters (`max_latency=30`) and the event pipeline are internal to the crate
with no `keyboard.toml` seam for hold-back or reorder logic.

## Goals / Non-Goals

**Goals:**
- Reduce cross-half key misordering for typical typing speeds (inter-key gaps ≥ 20–30 ms).
- Leave the peripheral binary unchanged in behaviour and timing.
- Keep `just check` and `just build` compiling the same central binary (no cargo-feature gate).
- Self-contained: no GitHub fork; vendored RMK committed to the repo.

**Non-Goals:**
- Eliminating all possible misordering. BLE latency is variable (0–7.5 ms+); a fixed
  hold-back is a heuristic that reduces, not eliminates, the race window. Residual flips
  at very fast inter-hand rolls remain possible.
- Changing connection parameters (`max_latency`, interval). Per Nordic SoftDevice docs,
  slave latency delays central→peripheral packets only; it does not affect the
  peripheral→central keypress notification path and is therefore not the primary lever.
- Supporting merge upstream — the vendor tree is a local patch and is not intended for PR.

## Decisions

### D1 — Central hold-back, not a reorder buffer
A reorder buffer (timestamp + merge-sort) would require adding a timestamp field to
`KeyboardEvent` (a wire-format change touching both binaries and the split protocol),
threading it through the entire keyboard state machine, and choosing a hold window long
enough to absorb worst-case BLE jitter. Cost is high and the interaction with tap-hold
timing is complex.

A fixed pre-publish delay on the central's own matrix events is the approach ZMK documents
for the identical race (`(min+max)/2/2 ≈ 3.75 ms`). It shifts the central stream to roughly
align with the peripheral stream's average arrival, at the cost of a small, uniform, and
imperceptible latency addition to every left-hand keypress. Chosen because: low code
surface, no wire-format change, no interaction with tap-hold state machines, and already
validated by ZMK on the same hardware.

Default: **3750 µs**. Tuning ladder if residual flips are observed: 6000 µs → 8000 µs.
Do not exceed ~12000 µs (beyond that, left-hand latency becomes perceptible).

### D2 — Gated by construction (Matrix field + codegen), not by cargo feature
A `#[cfg(feature = "central_holdback")]` gate would make `just check` (justfile:71-73, plain
`cargo build --bin central` with no features) compile a different binary than `just build`
(which goes through `cargo make uf2-central`). The sanity path would silently omit the fix.

Instead, the hold-back value is set at **construction time** via a builder method
(`with_hold_back_micros`) appended only in the `rmk_central` proc-macro codegen site
(`orchestrator.rs` Normal and DirectPin central arms). The peripheral codegen is untouched;
both binaries share the same compiled `Matrix::run()`. `check == build` by construction.

### D3 — Vendor + `[patch]`, not a GitHub fork
A fork requires an external GitHub account and introduces a dependency on repository
availability. Vendoring the entire RMK workspace at the exact pinned rev into `vendor/rmk/`
is fully self-contained, committed to the repo, reproducible on any machine without network
access after `git clone`. The `[patch."https://github.com/HaoboGu/rmk"]` directive points
only at `rmk` (the crate the user depends on); sibling crates (`rmk-macro`, `rmk-types`,
`rmk-config`) resolve through RMK's own `../` relative path-deps and require no separate
patch entries.

## Risks / Trade-offs

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Residual flips persist at 3750 µs | Medium (fast typists) | Tuning ladder: raise to 6000 → 8000 µs via the two `with_hold_back_micros` calls in orchestrator.rs; rebuild + hardware test. |
| vendor/rmk/ adds ~several MB to the repo | Certain | Accepted. Removes network dependency; keeps the repo self-contained. Only docs/, examples/, .git removed to trim size. |
| RMK upstream changes make re-vendoring difficult | Low (rev is pinned) | The vendored tree is at a fixed revision. Future RMK bumps require re-vendoring and re-applying the two file edits; the change is small and locatable. |
| Hold-back stalls central scan loop during delay | Low | `Timer::after` yields to the embassy executor; other tasks (BLE stack, display) continue running. At 3750 µs the loop rate is 266 events/sec, well above human typing speed. Debounce window (20 ms) ensures no key state is missed. |
| `just check` / `just build` divergence from future Makefile changes | Low | D2 eliminates the current divergence path; document the mechanism in DESIGN.md so future contributors don't add a feature gate. |
