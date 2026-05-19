use std::any::Any;

use gs_core::bridge::ValueConverter;
use gs_core::core::types::{BooleanType, TypeContext, UndefinedType, Value};

use super::types::number::NumberType;
use super::types::string_type::StringType;

pub struct GscriptValueConverter;

impl ValueConverter for GscriptValueConverter {
    fn convert_from_gvm(&self, context: &dyn TypeContext, value: &Value) -> Box<dyn Any> {
        match value.type_.name() {
            "Undefined" => Box::new(()),
            "String" => {
                let s = context
                    .get_string(value.value)
                    .unwrap_or("")
                    .to_string();
                Box::new(s)
            }
            "Number" => Box::new(value.value),
            "Boolean" => Box::new(value.value > 0),
            _ => Box::new(value.value),
        }
    }

    fn convert_from_gvm_to(
        &self,
        context: &dyn TypeContext,
        value: &Value,
        target: &str,
    ) -> Box<dyn Any> {
        match target {
            "String" => {
                let s = context
                    .get_string(value.value)
                    .unwrap_or("")
                    .to_string();
                Box::new(s)
            }
            "i32" | "Number" => Box::new(value.value),
            "bool" | "Boolean" => Box::new(value.value > 0),
            _ => self.convert_from_gvm(context, value),
        }
    }

    fn convert_to_gvm(&self, context: &mut dyn TypeContext, value: Box<dyn Any>) -> Value {
        if let Some(s) = value.downcast_ref::<String>() {
            let index = context.add_string(s);
            Value::new(index, Box::new(StringType))
        } else if let Some(i) = value.downcast_ref::<i32>() {
            Value::new(*i, Box::new(NumberType))
        } else if let Some(b) = value.downcast_ref::<bool>() {
            Value::new(if *b { 1 } else { 0 }, Box::new(BooleanType))
        } else {
            Value::new(0, Box::new(UndefinedType))
        }
    }
}
