# YUYUDHAN patch: per-profile whitelist advertising

Vendored RMK at upstream rev `b98204928e9a8532064ce99add1f6c4c554e08c9`
(https://github.com/HaoboGu/rmk), pristine except for the single patch below.
Wired via `[patch."https://github.com/HaoboGu/rmk"]` in the repo-root
`Cargo.toml`; patching `rmk` alone cascades to its `../rmk-macro`,
`../rmk-types`, `../rmk-config` path-deps inside this tree.

## Problem

Stock RMK advertises ONE identical, undirected, unfiltered BLE identity for
every profile (`rmk/src/ble/mod.rs`, `ConnectableScannableUndirected`, default
`AdvFilterPolicy::Unfiltered`). A profile only selects which LTK is loaded.
With two bonded computers in range, whichever host answers the advertisement
first wins the connection — switching to profile 0 lets the profile-1 computer
steal the slot, encryption fails against the wrong LTK, and the keyboard sits
in a dead unencrypted connection while the OLED shows `~ P0` indefinitely.
Upstream master (checked 2026-07-02) has no fix.

## Fix (ZMK model)

`rmk/src/ble/mod.rs` only:

- `advertise()` takes a new `peer: Option<Address>` parameter. When `Some`,
  the peer is loaded into the controller filter accept list
  (`Peripheral::set_filter_accept_list`) and advertising runs with
  `AdvFilterPolicy::FilterConnAndScan` — the keyboard is invisible and
  unconnectable to every other host. When `None`, the list is cleared and
  advertising is `Unfiltered` (open pairing, stock behavior).
- The connection loop computes `peer` on every iteration from
  `ProfileManager::active_bond_info()`, guarded on `identity.irk.is_some()`:
  a bond without an IRK could belong to an RPA-rotating host the controller
  cannot resolve, so such profiles keep advertising open rather than risking
  a permanent lock-out.
- `BleTransport` bounds widened with
  `ControllerCmdSync<LeClearFilterAcceptList> + ControllerCmdSync<LeAddDeviceToFilterAcceptList>`
  (both implemented by nrf-sdc's SoftDevice Controller).

Rotating private addresses (macOS/Windows) are handled by trouble-host, which
auto-syncs the controller resolving list from bond IRKs with
`PrivacyMode::Device` whenever bond information changes.

Behavioral rule this introduces: **a bonded profile is invisible to other
hosts.** To pair a profile to a new computer, clear it first (User6 /
ESC+O on the MEDIA layer) — that returns the profile to open advertising.

## Files touched

- `rmk/src/ble/mod.rs` — full delta in `YUYUDHAN_PATCH.patch`.

## Regenerate the diff

```sh
diff -u ~/.cargo/git/checkouts/rmk-cd9707f7f2031ce2/b982049/rmk/src/ble/mod.rs \
        vendor/rmk/rmk/src/ble/mod.rs > vendor/rmk/YUYUDHAN_PATCH.patch
```

## Vendor layout note

Only `rmk/`, `rmk-types/`, `rmk-macro/`, `rmk-config/`, licenses and the root
`README.md` are vendored (no docs/, examples/, .github/). `rmk/README.md` is a
symlink to `../README.md`, hence the root copy — `rmk/src/lib.rs` includes it
via `#![doc = include_str!("../README.md")]`.
