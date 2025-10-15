use std::collections::HashMap;

use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectRef, ObjectDescriptor};

pub struct TypeInfo {
    pub factory: fn(args: &[Value]) -> Result<ObjectRef>,
    pub descriptor: fn() -> ObjectDescriptor,
    pub constants: fn() -> Vec<(String, Value)>,
}

#[derive(Default)]
pub struct Registry {
    types: HashMap<String, TypeInfo>,
}

impl Registry {
    pub fn new() -> Self { Self { types: HashMap::new() } }
    pub fn register(&mut self, type_name: &str, info: TypeInfo) {
        self.types.insert(type_name.to_string(), info);
    }
    pub fn has_type(&self, type_name: &str) -> bool {
        self.types.contains_key(&type_name.to_ascii_uppercase()) || self.types.contains_key(type_name)
    }
    pub fn make(&self, type_name: &str, args: &[Value]) -> Result<ObjectRef> {
        let key1 = type_name.to_string();
        let key2 = type_name.to_ascii_uppercase();
        let info = self.types.get(&key1).or_else(|| self.types.get(&key2))
            .ok_or_else(|| BasilError(format!("Type '{}' not available; rebuild with appropriate Cargo features.", type_name)))?;
        (info.factory)(args)
    }
    pub fn describe_type(&self, type_name: &str) -> Result<ObjectDescriptor> {
        let key1 = type_name.to_string();
        let key2 = type_name.to_ascii_uppercase();
        let info = self.types.get(&key1).or_else(|| self.types.get(&key2))
            .ok_or_else(|| BasilError(format!("Type '{}' not available; rebuild with appropriate Cargo features.", type_name)))?;
        Ok((info.descriptor)())
    }
    pub fn all_constants(&self) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        for info in self.types.values() {
            out.extend((info.constants)());
        }
        out
    }
}

// Describe this function: it registers all the objects in the registry.
// This is the only function that should be called from the compiler.
// The compiler will call this function with the appropriate registry.
// The compiler will also call this function with a null registry,
// which is used for testing.

// Example: cargo run --features obj-bmx-rider --features obj-bmx-team -- run examples\objects.basil
// Example: cargo run -q -p basilc --features obj-bmx -- run examples\objects.basil


pub fn register_objects(_reg: &mut Registry) {
    // conditional registrations
    #[cfg(feature = "obj-curl")]
    {
        crate::curl::register(_reg);
    }
    #[cfg(feature = "obj-base64")]
    {
        crate::base64::register(_reg);
    }
    #[cfg(feature = "obj-bmx-rider")]
    {
        crate::bmx_rider::register(_reg);
    }
    #[cfg(feature = "obj-bmx-team")]
    {
        crate::bmx_team::register(_reg);
    }
    #[cfg(feature = "obj-zip")]
    {
        crate::zip::register(_reg);
    }
    #[cfg(feature = "obj-csv")]
    {
        crate::csv::register(_reg);
    }
    #[cfg(feature = "obj-sqlite")]
    {
        crate::sqlite::register(_reg);
    }
    #[cfg(feature = "obj-audio")]
    {
        crate::audio::register(_reg);
    }
    #[cfg(feature = "obj-midi")]
    {
        crate::midi::register(_reg);
    }
    #[cfg(feature = "obj-daw")]
    {
        crate::daw::register(_reg);
    }
    #[cfg(feature = "obj-ai")]
    {
        crate::ai::register(_reg);
    }
    #[cfg(feature = "obj-term")]
    {
        crate::term::register(_reg);
    }
    #[cfg(any(feature = "obj-aws-s3", feature = "obj-aws-ses", feature = "obj-aws-sqs"))]
    {
        // Bridge registrations from basil-objects-aws crate
        let mut add = |type_name: &str, info: basil_objects_aws::TypeInfo| {
            _reg.register(type_name, TypeInfo { factory: info.factory, descriptor: info.descriptor, constants: info.constants });
        };
        basil_objects_aws::register(&mut add);
    }
    #[cfg(any(feature = "obj-net-sftp", feature = "obj-net-smtp", feature = "obj-net-http"))]
    {
        // Bridge registrations from basil-objects-net crate
        let mut add = |type_name: &str, info: basil_objects_net::TypeInfo| {
            _reg.register(type_name, TypeInfo { factory: info.factory, descriptor: info.descriptor, constants: info.constants });
        };
        basil_objects_net::register(&mut add);
    }
    #[cfg(feature = "obj-crypto-pgp")]
    {
        // Bridge registrations from basil-objects-crypto crate
        let mut add = |type_name: &str, info: basil_objects_crypto::TypeInfo| {
            _reg.register(type_name, TypeInfo { factory: info.factory, descriptor: info.descriptor, constants: info.constants });
        };
        basil_objects_crypto::register(&mut add);
    }
    #[cfg(any(feature = "obj-sql-mysql", feature = "obj-sql-postgres"))]
    {
        // Bridge registrations from basil-objects-sql crate
        let mut add = |type_name: &str, info: basil_objects_sql::TypeInfo| {
            _reg.register(type_name, TypeInfo { factory: info.factory, descriptor: info.descriptor, constants: info.constants });
        };
        basil_objects_sql::register(&mut add);
    }
}

#[cfg(feature = "obj-base64")]
mod base64;
#[cfg(feature = "obj-bmx-rider")]
mod bmx_rider;
#[cfg(feature = "obj-bmx-team")]
mod bmx_team;
#[cfg(feature = "obj-zip")]
pub mod zip;
#[cfg(feature = "obj-curl")]
pub mod curl;
#[cfg(feature = "obj-json")]
pub mod json;
#[cfg(feature = "obj-csv")]
mod csv;
#[cfg(feature = "obj-sqlite")]
pub mod sqlite;
#[cfg(feature = "obj-audio")]
pub mod audio;
#[cfg(feature = "obj-midi")]
pub mod midi;
#[cfg(feature = "obj-daw")]
pub mod daw;
#[cfg(feature = "obj-ai")]
pub mod ai;
#[cfg(feature = "obj-term")]
pub mod term;
