pub mod error;
pub mod ir;
pub mod manifest;
pub mod validation;

pub use error::{AtomicRuntimeError, AtomicVmState, ATOMIC_RUNTIME_ERROR_TYPE};
pub use ir::{AtomicGameProgram, AtomicInstruction, ATOMIC_GAME_IR_FORMAT, ATOMIC_GAME_IR_VERSION};
pub use manifest::{
    AtomicGameManifest, ATOMIC_GAME_FORMAT, ATOMIC_GAME_FORMAT_VERSION, ATOMIC_GAME_RUNTIME_VERSION,
};
pub use validation::{validate_atomic_program, AtomicValidationError};
