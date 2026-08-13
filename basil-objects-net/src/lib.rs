use basil_bytecode::{ObjectDescriptor, ObjectRef, Value};
use basil_common::Result;

// Module gating
#[cfg(feature = "obj-net-http")]
pub mod http;
#[cfg(any(
    feature = "obj-net-sftp",
    feature = "obj-net-smtp",
    feature = "obj-net-http"
))]
mod runtime;
#[cfg(feature = "obj-net-sftp")]
pub mod sftp;
#[cfg(feature = "obj-net-smtp")]
pub mod smtp;

// Mirror the Registry interface expected by basil-objects as a shim
pub struct TypeInfo {
    pub factory: fn(args: &[Value]) -> Result<ObjectRef>,
    pub descriptor: fn() -> ObjectDescriptor,
    pub constants: fn() -> Vec<(String, Value)>,
}

// Bridge registration function called by basil-objects
pub fn register<F: FnMut(&str, TypeInfo)>(mut reg: F) {
    #[cfg(feature = "obj-net-sftp")]
    {
        sftp::register(&mut reg);
    }
    #[cfg(feature = "obj-net-smtp")]
    {
        smtp::register(&mut reg);
    }
    #[cfg(feature = "obj-net-http")]
    {
        http::register(&mut reg);
    }
}
