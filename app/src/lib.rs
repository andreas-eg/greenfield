//! Beargarden sk8r as an app: the lens, given Beargarden's game modes. The lens itself knows
//! no game; a board whose drive says `"sim": "sk8"` is ridden by `lens-sk8` here.
//! @req SODER-LENS-S260

/// Beargarden's game modes.
pub fn modes() -> Vec<lens_app::lens_mode::Registration> {
    vec![lens_sk8::registration()]
}

/// The iOS shell's entry, in place of the lens's own `lens_main`: the lens with Beargarden's
/// modes. winit starts UIApplicationMain from inside the event loop, so this never returns.
#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn beargarden_main() {
    lens_app::run_with(&modes());
}
