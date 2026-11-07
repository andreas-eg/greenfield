//! sk8: Greenfield's skate simulation.
//!
//! An implementation of the sk8r specification (`sk8t/skateboard.md`, "Skateboard game:
//! complete implementation specification") as a platform-free library: the virtual
//! controller and Flickit recognizer (B.4), the board physics (B.5 to B.9), grinding (B.10),
//! wipeout and respawn (B.13), scoring (B.14), difficulty (B.15) and the camera (B.16).
//!
//! The lens hosts it as the `skate` drive (greenfield/feature_requests.md, GF-REQ-LENS-1)
//! on iOS, macOS and the web; `play/` in this repository is a browser harness on the same
//! C ABI (`ffi.rs`). The realm (greenfield/world) is the authority for what is kept: it
//! hears the events this library emits and re-scores them.

pub mod build;
pub mod ffi;
pub mod frame;
pub mod grind;
pub mod input;
pub mod math;
pub mod scoring;
pub mod sim;
pub mod spots;
pub mod tuning;
pub mod world;

pub use frame::Frame;
pub use input::VirtualController;
pub use sim::{Event, Sim, State};
pub use tuning::{Difficulty, Tuning};
pub use world::World;
