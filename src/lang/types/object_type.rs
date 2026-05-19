use gs_core::core::types::{BooleanType, Operation, Type, TypeContext, UndefinedType, Value};

use crate::lang::plain_object::GvmPlainObject;

use super::array_object::ArrayObject;
use super::array_type::ArrayType;
use super::number::NumberType;
use super::string_type::StringType;

#[derive(Debug, Clone)]
pub struct ObjectType;

impl Type for ObjectType {
    fn name(&self) -> &str {
        "Object"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(op, Operation::New | Operation::Get | Operation::Eql)
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
                let obj_ref = context.heap_add_object_box(Box::new(GvmPlainObject::new()));
                Value::new(obj_ref, Box::new(ObjectType))
            }
            Operation::Get => {
                let field = other_value.expect("GET requires a field operand");
                if field.type_.name() == "String" {
                    let field_name = context
                        .get_string(field.value)
                        .unwrap_or("")
                        .to_string();
                    match field_name.as_str() {
                        "fields" => {
                            let keys = context.heap_get_object_keys(this_value.value);
                            let mut array_obj = ArrayObject::new();
                            for (i, key) in keys.iter().enumerate() {
                                let key_ref = context.add_string(key);
                                array_obj.set_value_by_index(i as i32,
                                    Value::with_comment(key_ref, Box::new(StringType), "Reflection based value".to_string()));
                            }
                            let array_ref = context.heap_add_object_box(Box::new(array_obj));
                            Value::new(array_ref, Box::new(ArrayType))
                        }
                        "ref" => Value::new(this_value.value, Box::new(NumberType)),
                        _ => {
                            match context.heap_get_object_value(this_value.value, &field_name) {
                                Some(v) => v,
                                None => Value::new(0, Box::new(UndefinedType)),
                            }
                        }
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
            _ => panic!("Operation {:?} not supported on Object", op),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(ObjectType)
    }
}
