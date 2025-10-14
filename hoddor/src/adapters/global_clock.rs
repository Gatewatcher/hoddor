use crate::ports::clock::ClockPort;
use lazy_static::lazy_static;

#[cfg(target_arch = "wasm32")]
use crate::adapters::wasm::Clock;
#[cfg(not(target_arch = "wasm32"))]
use crate::adapters::native::Clock;

lazy_static! {
    pub static ref CLOCK: Clock = Clock::new();
}

/// Returns a reference to the global clock instance
pub fn clock() -> &'static dyn ClockPort {
    &*CLOCK
}
