//! Physically based simulation SOPs.
//!
//! These operators evolve geometry over time. Unlike most SOPs they are
//! conceptually stateful, but they expose both a stateless SOP front-end
//! (re-simulating from rest to a requested frame) and a reusable stateful
//! solver for efficient interactive playback.

pub mod softbody;

pub use softbody::{SoftbodyParams, SoftbodySolver, SoftbodySop};
