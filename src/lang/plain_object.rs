use std::collections::HashMap;

use gs_core::core::object::GvmObject;
use gs_core::core::types::{UndefinedType, Value};

#[derive(Debug, Clone)]
pub struct GvmPlainObject {
    values: HashMap<String, Value>,
}

impl GvmPlainObject {
    pub fn new() -> Self {
        GvmPlainObject {
            values: HashMap::new(),
        }
    }

    /// Direct setter that avoids going through the GvmObject trait.
    pub fn set_value_direct(&mut self, id: &str, v: Value) {
        self.values.insert(id.to_string(), v);
    }
}

impl Default for GvmPlainObject {
    fn default() -> Self {
        Self::new()
    }
}

impl GvmObject for GvmPlainObject {
    fn set_value(&mut self, id: &str, v: Value) {
        self.values.insert(id.to_string(), v);
    }

    fn get_value(&self, id: &str) -> Option<Value> {
        match self.values.get(id) {
            Some(v) => Some(v.clone()),
            None => Some(Value::new(0, Box::new(UndefinedType))),
        }
    }

    fn has_value(&self, id: &str) -> bool {
        self.values.contains_key(id)
    }

    fn get_values(&self) -> Vec<Value> {
        self.values.values().cloned().collect()
    }

    fn get_keys(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    fn pre_destroy(&self) {}

    fn clone_object(&self) -> Box<dyn GvmObject> {
        Box::new(self.clone())
    }
}
