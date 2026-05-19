use gs_core::core::types::{Operation, Type, TypeContext, UndefinedType, Value};

use super::array_object::ArrayObject;
use super::number::NumberType;

#[derive(Debug, Clone)]
pub struct ArrayType;

impl Type for ArrayType {
    fn name(&self) -> &str {
        "Array"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(op, Operation::New | Operation::Get | Operation::Add)
    }

    fn perform(
        &self,
        context: &mut dyn TypeContext,
        op: Operation,
        this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value {
        match op {
            Operation::New => {
                let obj_ref = context.heap_add_object_box(Box::new(ArrayObject::new()));
                Value::new(obj_ref, Box::new(ArrayType))
            }
            Operation::Add => {
                let other = other_value.expect("ADD requires two operands");
                let length = context
                    .heap_get_object_value(this_value.value, "length")
                    .map(|v| v.value)
                    .unwrap_or(0);
                let idx_str = length.to_string();
                context.heap_set_object_value(this_value.value, &idx_str, other.clone());
                Value::new(this_value.value, Box::new(ArrayType))
            }
            Operation::Get => {
                let field = other_value.expect("GET requires a field operand");
                match field.type_.name() {
                    "String" => {
                        let field_name = context
                            .get_string(field.value)
                            .unwrap_or("")
                            .to_string();
                        match field_name.as_str() {
                            "length" => {
                                // Array length is stored as a property on the heap object.
                                // The ArrayObject tracks its own length, but we access
                                // it through the heap interface.
                                match context.heap_get_object_value(this_value.value, "length") {
                                    Some(v) => v,
                                    None => Value::new(0, Box::new(NumberType)),
                                }
                            }
                            _ => Value::new(0, Box::new(UndefinedType)),
                        }
                    }
                    "Number" => {
                        // Index-based access
                        let index_str = field.value.to_string();
                        match context.heap_get_object_value(this_value.value, &index_str) {
                            Some(v) => v,
                            None => Value::undefined(),
                        }
                    }
                    _ => Value::new(0, Box::new(UndefinedType)),
                }
            }
            _ => panic!("Operation {:?} not supported on Array", op),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(ArrayType)
    }
}
