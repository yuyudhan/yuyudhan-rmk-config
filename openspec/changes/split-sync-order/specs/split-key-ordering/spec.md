## ADDED Requirements

### Requirement: Central hold-back delay

The central (left) half's matrix scanner MUST apply a configurable pre-publish delay
(hold-back) before emitting each key event into the shared `KeyboardEvent` channel.
The delay MUST default to 0 (no effect) when not configured, so the peripheral binary
and unibody keyboards are unaffected.

The hold-back value for the split central binary is set at compile time via the
`rmk_central` proc-macro codegen. The initial value is 3750 µs. If residual
cross-half misordering is observed during hardware testing, the value should be raised
incrementally (6000 µs → 8000 µs) until misordering is no longer observable during
normal typing. Values above 12000 µs are not permitted (left-hand latency becomes
perceptible).

#### Scenario: Normal cross-hand roll

- **WHEN** a key on the left half and a key on the right half are pressed within the
  same BLE connection interval (~7.5 ms) during normal typing speeds (inter-key gap
  ≥ 20 ms)
- **THEN** the key events are delivered to the host in physical press order the majority
  of the time

#### Scenario: Peripheral binary unaffected

- **WHEN** the peripheral (right half) binary is compiled with the same RMK codebase
- **THEN** its `Matrix` hold-back value is 0 and its key-event publish latency is
  unchanged from baseline

#### Scenario: Single-half typing unaffected

- **WHEN** all keypresses originate from only one half
- **THEN** key ordering is identical to pre-change behaviour (hold-back shifts all events
  uniformly, preserving intra-half order)

### Requirement: Tunable hold-back with documented ladder

The hold-back value MUST be a single named constant or literal in the proc-macro codegen
(`orchestrator.rs`) that an implementer can change without understanding the broader
codebase. The source location and tuning ladder (3750 → 6000 → 8000 µs, max 12000 µs)
MUST be documented in a code comment at the definition site.

#### Scenario: Implementer adjusts hold-back

- **WHEN** residual misordering is observed after flashing
- **THEN** the implementer can locate the two `with_hold_back_micros(3750)` calls in
  `vendor/rmk/rmk-macro/src/codegen/orchestrator.rs`, change the value, run
  `just build both`, and reflash without any other changes

### Requirement: check == build parity

The `just check` recipe (plain `cargo build --bin central/peripheral`) MUST compile the
same central binary — including the hold-back delay — as `just build`. No cargo feature
flag may gate the hold-back; it MUST be injected purely via construction-time codegen.

#### Scenario: Sanity check matches flash

- **WHEN** `just check left` succeeds
- **THEN** the compiled binary has the same hold-back delay as the binary produced by
  `just build left`
