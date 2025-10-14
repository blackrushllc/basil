use basil_common::Result;
use basil_bytecode::{ObjectDescriptor, Value, ObjectRef};

#[cfg(any(feature = "obj-aws-s3", feature = "obj-aws-ses", feature = "obj-aws-sqs"))]
mod runtime;
#[cfg(feature = "obj-aws-s3")]
pub mod s3;
#[cfg(feature = "obj-aws-ses")]
pub mod ses;
#[cfg(feature = "obj-aws-sqs")]
pub mod sqs;
#[cfg(any(feature = "obj-aws-s3", feature = "obj-aws-ses", feature = "obj-aws-sqs"))]
pub mod context;

// Reuse the same Registry and TypeInfo types from basil-objects by declaring a minimal mirror here.
// The basil-objects crate will call register() below and pass its Registry.

pub struct TypeInfo {
    pub factory: fn(args: &[Value]) -> Result<ObjectRef>,
    pub descriptor: fn() -> ObjectDescriptor,
    pub constants: fn() -> Vec<(String, Value)>,
}

pub struct RegistryShim<'a> {
    inner: &'a mut dyn FnMut(&str, TypeInfo),
}

impl<'a> RegistryShim<'a> {
    pub fn new<F: FnMut(&str, TypeInfo) + 'a>(f: &'a mut F) -> Self { Self { inner: f } }
    pub fn register(&mut self, type_name: &str, info: TypeInfo) {
        (self.inner)(type_name, info);
    }
}

// This function will be called from basil-objects when corresponding features are enabled.
pub fn register<F: FnMut(&str, TypeInfo)>(mut reg: F) {
    // Context object (AWS@)
    #[cfg(any(feature = "obj-aws-s3", feature = "obj-aws-ses", feature = "obj-aws-sqs"))]
    {
        context::register(&mut reg);
    }
    // Service objects
    #[cfg(feature = "obj-aws-s3")]
    {
        s3::register(&mut reg);
    }
    #[cfg(feature = "obj-aws-ses")]
    {
        ses::register(&mut reg);
    }
    #[cfg(feature = "obj-aws-sqs")]
    {
        sqs::register(&mut reg);
    }
}
