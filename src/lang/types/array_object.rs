use gs_core::core::object::GvmObject;
use gs_core::core::types::Value;

#[derive(Debug, Clone)]
pub struct ArrayObject {
    elements: Vec<Option<Value>>,
}

impl ArrayObject {
    pub fn new() -> Self {
        ArrayObject {
            elements: Vec::new(),
        }
    }

    pub fn set_value_by_index(&mut self, index: i32, v: Value) {
        let idx = index as usize;
        if idx >= self.elements.len() {
            self.elements.resize_with(idx + 1, || None);
        }
        self.elements[idx] = Some(v);
    }

    pub fn get_value_by_index(&self, index: i32) -> Value {
        let idx = index as usize;
        if idx < self.elements.len() {
            match &self.elements[idx] {
                Some(v) => v.clone(),
                None => Value::undefined(),
            }
        } else {
            Value::undefined()
        }
    }

    pub fn get_length(&self) -> i32 {
        self.elements.len() as i32
    }
}

impl Default for ArrayObject {
    fn default() -> Self {
        Self::new()
    }
}

impl GvmObject for ArrayObject {
    fn set_value(&mut self, id: &str, v: Value) {
        if let Ok(idx) = id.parse::<i32>() {
            self.set_value_by_index(idx, v);
        }
    }

    fn get_value(&self, id: &str) -> Option<Value> {
        if id == "length" {
            Some(Value::new(self.get_length(), Box::new(super::number::NumberType)))
        } else if let Ok(idx) = id.parse::<i32>() {
            let val = self.get_value_by_index(idx);
            if val.type_.name() == "Undefined" { None } else { Some(val) }
        } else {
            None
        }
    }

    fn has_value(&self, id: &str) -> bool {
        if id == "length" {
            true
        } else if let Ok(idx) = id.parse::<usize>() {
            idx < self.elements.len() && self.elements[idx].is_some()
        } else {
            false
        }
    }

    fn get_values(&self) -> Vec<Value> {
        self.elements.iter().filter_map(|e| e.clone()).collect()
    }

    fn get_keys(&self) -> Vec<String> {
        (0..self.elements.len())
            .filter(|i| self.elements[*i].is_some())
            .map(|i| i.to_string())
            .collect()
    }

    fn pre_destroy(&self) {}

    fn clone_object(&self) -> Box<dyn GvmObject> {
        Box::new(self.clone())
    }
}
