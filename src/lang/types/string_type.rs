use gs_core::core::types::{BooleanType, Operation, Type, TypeContext, UndefinedType, Value};

use super::number::NumberType;
use crate::lang::native_object::marshal_native_to_value;

#[derive(Debug, Clone)]
pub struct StringType;

impl Type for StringType {
    fn name(&self) -> &str {
        "String"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(op, Operation::Add | Operation::Get | Operation::Eql)
    }

    fn perform(
        &self,
        context: &mut dyn TypeContext,
        op: Operation,
        this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value {
        match op {
            Operation::Add => {
                let other = other_value.expect("ADD requires two operands");
                let this_str = context
                    .get_string(this_value.value)
                    .unwrap_or("")
                    .to_string();

                let other_str = match other.type_.name() {
                    "String" => context
                        .get_string(other.value)
                        .unwrap_or("")
                        .to_string(),
                    "Number" => other.value.to_string(),
                    "Boolean" => {
                        if other.value > 0 {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    }
                    _ => String::new(),
                };

                let result = format!("{}{}", this_str, other_str);
                let idx = context.add_string(&result);
                Value::new(idx, Box::new(StringType))
            }
            Operation::Get => {
                let field = other_value.expect("GET requires a field operand");
                if field.type_.name() == "Number" {
                    let s = context.get_string(this_value.value).unwrap_or("");
                    let idx = field.value as usize;
                    if idx < s.len() {
                        let ch = s[idx..].chars().next().unwrap().to_string();
                        let str_idx = context.add_string(&ch);
                        return Value::new(str_idx, Box::new(StringType));
                    }
                    return Value::undefined();
                } else if field.type_.name() == "String" {
                    let field_name = context
                        .get_string(field.value)
                        .unwrap_or("")
                        .to_string();
                    match field_name.as_str() {
                        "lowercase" => {
                            let s = context
                                .get_string(this_value.value)
                                .unwrap_or("")
                                .to_lowercase();
                            let idx = context.add_string(&s);
                            Value::new(idx, Box::new(StringType))
                        }
                        "length" => {
                            let len = context
                                .get_string(this_value.value)
                                .map(|s| s.len() as i32)
                                .unwrap_or(0);
                            Value::new(len, Box::new(NumberType))
                        }
                        "bytes" => {
                            let s = context
                                .get_string(this_value.value)
                                .unwrap_or("")
                                .to_string();
                            use gs_core::bridge::native_module::NativeValue;
                            marshal_native_to_value(NativeValue::Bytes(s.into_bytes()), context)
                        }
                        "ref" => Value::new(this_value.value, Box::new(NumberType)),
                        _ => Value::new(0, Box::new(UndefinedType)),
                    }
                } else {
                    Value::new(0, Box::new(UndefinedType))
                }
            }
            Operation::Eql => {
                let other = other_value.expect("EQL requires two operands");
                let result = this_value.value == other.value;
                Value::new(i32::from(result), Box::new(BooleanType))
            }
            _ => panic!("Operation {:?} not supported on String", op),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(StringType)
    }
}
