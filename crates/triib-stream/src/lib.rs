//! AVTP audio streams for triib's talker and listener entities: AAF and
//! AM824 frames built and read (IEEE 1722-2016, 7 and 6.4), sent paced on
//! gPTP time and received into a buffer the audio device plays, with
//! audio from and to the computer's devices through cpal, a test tone or
//! nothing.
//!
//! The media clock is gPTP's: a talker sends its samples at the nominal
//! rate of gPTP time, read from the interface's PTP hardware clock, and
//! the audio device, on its own clock, is kept in step by dropping or
//! repeating a sample frame now and then.

pub mod aaf;
pub mod am824;
pub mod audio;
pub mod clock;
pub mod listener;
pub mod media;
pub mod realtime;
pub mod talker;

pub use clock::MediaClock;
pub use listener::{Listener, ListenerConfig, ListenerStats};
pub use talker::{Talker, TalkerConfig, TalkerStats};
