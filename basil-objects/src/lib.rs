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

pub fn register_objects(_reg: &mut Registry) {
    // conditional registrations
    #[cfg(feature = "obj-bmx-rider")]
    {
        crate::bmx_rider::register(_reg);
    }
    #[cfg(feature = "obj-bmx-team")]
    {
        crate::bmx_team::register(_reg);
    }
}

#[cfg(feature = "obj-bmx-rider")]
mod bmx_rider;
#[cfg(feature = "obj-bmx-team")]
mod bmx_team;
