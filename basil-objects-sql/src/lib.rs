use basil_common::Result;
use basil_bytecode::{ObjectDescriptor, Value, ObjectRef};

#[cfg(feature = "obj-sql-mysql")]
pub mod mysql;
#[cfg(feature = "obj-sql-postgres")]
pub mod postgres;
#[cfg(any(feature = "obj-sql-mysql", feature = "obj-sql-postgres"))]
pub mod runtime;

// Mirror the TypeInfo used by basil-objects for bridging registrations.
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

// Called from basil-objects when feature is enabled.
pub fn register<F: FnMut(&str, TypeInfo)>(mut reg: F) {
    #[cfg(feature = "obj-sql-mysql")]
    {
        mysql::register(&mut reg);
    }
    #[cfg(feature = "obj-sql-postgres")]
    {
        postgres::register(&mut reg);
    }
}
