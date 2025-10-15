use crate::ports::PersistencePort;
use lazy_static::lazy_static;

#[cfg(target_arch = "wasm32")]
use crate::adapters::wasm::Persistence;
#[cfg(not(target_arch = "wasm32"))]
use crate::adapters::native::Persistence;

lazy_static! {
    pub static ref PERSISTENCE: Persistence = Persistence::new();
}

/// Returns a reference to the global persistence instance
pub fn persistence() -> &'static dyn PersistencePort {
    &*PERSISTENCE
}
