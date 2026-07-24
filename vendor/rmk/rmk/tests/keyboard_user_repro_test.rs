/// Repro tests for user-reported issues with HRM profile:
/// flow_tap + unilateral_tap + permissive_hold + 250ms hold timeout.
///
/// Layout (1 row, 5 cols): [A, mt!(B,LShift), mt!(C,LGui), lt!(1,D), mt!(E,LAlt)]
/// Hands: [Left, Right, Right, Right, Right]
///   - col 0: plain A (Left)      ≈ user's "a" / opposite-hand trigger key
///   - col 2: mt!(C, LGui) Right  ≈ user's K (mod on hold)
///   - col 4: mt!(E, LAlt) Right  ≈ user's L (mod on hold)
pub mod common;

use embassy_time::Duration;
use rmk::config::{BehaviorConfig, Hand, MorsesConfig};
use rmk::keyboard::Keyboard;
use rmk::types::morse::{MorseMode, MorseProfile};

use crate::common::morse::create_morse_keyboard;

const KC_LGUI: u8 = 0b0000_1000;
const KC_LALT: u8 = 0b0000_0100;

fn create_hrm_keyboard() -> Keyboard<'static> {
    let hand = [[Hand::Left, Hand::Right, Hand::Right, Hand::Right, Hand::Right]];
    create_morse_keyboard(
        BehaviorConfig {
            morse: MorsesConfig {
                enable_flow_tap: true,
                prior_idle_time: Duration::from_millis(120),
                default_profile: MorseProfile::new(
                    Some(true), // unilateral_tap
                    Some(MorseMode::PermissiveHold),
                    Some(250u16),
                    Some(250u16),
                ),
                ..Default::default()
            },
            ..Default::default()
        },
        hand,
    )
}

/// User case 1: hold two same-hand mods (K+L), tap opposite-hand key (A).
/// Expected: LGui+LAlt+A chord, NOT letters.
#[test]
fn test_two_same_hand_mods_plus_cross_hand_key() {
    key_sequence_test! {
        keyboard: create_hrm_keyboard(),
        sequence: [
            [0, 2, true, 300],  // Press mt!(C, LGui)  ("K")
            [0, 4, true, 30],   // Press mt!(E, LAlt)  ("L"), same hand, both held
            [0, 0, true, 30],   // Press A (opposite hand)
            [0, 0, false, 30],  // Release A -> permissive hold resolves both mods
            [0, 4, false, 10],  // Release mt!(E, LAlt)
            [0, 2, false, 10],  // Release mt!(C, LGui)
        ],
        expected_reports: [
            [KC_LGUI, [0, 0, 0, 0, 0, 0]],                    // C -> LGui (permissive)
            [KC_LGUI | KC_LALT, [0, 0, 0, 0, 0, 0]],          // E -> LAlt (permissive)
            [KC_LGUI | KC_LALT, [kc_to_u8!(A), 0, 0, 0, 0, 0]], // A pressed under mods
            [KC_LGUI | KC_LALT, [0, 0, 0, 0, 0, 0]],          // A released
            [KC_LGUI, [0, 0, 0, 0, 0, 0]],                    // LAlt released
            [0, [0, 0, 0, 0, 0, 0]],                          // LGui released
        ]
    };
}

/// User case 2: fast cross-hand roll, mod-key first ("am"/"ankur" start).
/// Press mt!(C,LGui) (Right), roll into A (Left) with overlap, release in press order.
/// Expected: output "c a" in correct order, no mods.
#[test]
fn test_fast_cross_hand_roll_preserves_order() {
    key_sequence_test! {
        keyboard: create_hrm_keyboard(),
        sequence: [
            [0, 2, true, 300],  // Press mt!(C, LGui) ("a" of "am") - no flow tap (idle before)
            [0, 0, true, 50],   // Press A ("m") while C undecided -> must be buffered
            [0, 2, false, 50],  // Release C first (normal roll) -> tap 'c'
            [0, 0, false, 30],  // Release A
        ],
        expected_reports: [
            [0, [kc_to_u8!(C), 0, 0, 0, 0, 0]],               // 'c' first
            [0, [kc_to_u8!(C), kc_to_u8!(A), 0, 0, 0, 0]],    // then 'a' - order preserved
            [0, [0, kc_to_u8!(A), 0, 0, 0, 0]],               // release c (A keeps its slot)
            [0, [0, 0, 0, 0, 0, 0]],                          // release a
        ]
    };
}

/// User case 3: same-hand roll with mod keys ("lk"/"kl").
/// Hold mt!(E,LAlt) ("L"), press mt!(C,LGui) ("K"), release K then L, all fast.
/// Expected: letters "e c", NO Alt, NO Gui (unilateral tap protection).
#[test]
fn test_same_hand_mod_roll_yields_letters() {
    key_sequence_test! {
        keyboard: create_hrm_keyboard(),
        sequence: [
            [0, 4, true, 300],  // Press mt!(E, LAlt) ("L")
            [0, 2, true, 50],   // Press mt!(C, LGui) ("K"), same hand
            [0, 2, false, 50],  // Release K -> unilateral tap resolves both as letters
            [0, 4, false, 30],  // Release L
        ],
        expected_reports: [
            [0, [kc_to_u8!(E), 0, 0, 0, 0, 0]],               // 'e' first (unilateral tap)
            [0, [kc_to_u8!(E), kc_to_u8!(C), 0, 0, 0, 0]],    // then 'c'
            [0, [kc_to_u8!(E), 0, 0, 0, 0, 0]],               // release c
            [0, [0, 0, 0, 0, 0, 0]],                          // release e
        ]
    };
}
