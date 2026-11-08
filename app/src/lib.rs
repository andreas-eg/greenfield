//! Beargarden sk8r as an app: the lens, given Beargarden's game modes. The lens itself knows
//! no game; a board whose drive says `"sim": "sk8"` (a scooter's `"sk8_scooter"`) is ridden by
//! `lens-sk8` here.
//! @req SODER-LENS-S260

/// Beargarden's game modes: the board, and the scooter (SODER-LENS-S291).
pub fn modes() -> Vec<lens_app::lens_mode::Registration> {
    vec![lens_sk8::registration(), lens_sk8::scooter_registration()]
}

/// The iOS shell's entry, in place of the lens's own `lens_main`: the lens with Beargarden's
/// modes. winit starts UIApplicationMain from inside the event loop, so this never returns.
#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn beargarden_main() {
    lens_app::run_with(&modes());
}
