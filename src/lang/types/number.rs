use gs_core::core::types::{BooleanType, Operation, Type, TypeContext, Value};

use super::string_type::StringType;

#[derive(Debug, Clone)]
pub struct NumberType;

impl Type for NumberType {
    fn name(&self) -> &str {
        "Number"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(
            op,
            Operation::Add
                | Operation::Sub
                | Operation::Mult
                | Operation::Div
                | Operation::Mod
                | Operation::Not
                | Operation::Eql
                | Operation::Gt
                | Operation::Lt
        )
    }

    fn perform(
        &self,
        _context: &mut dyn TypeContext,
        op: Operation,
        this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value {
        match op {
            Operation::Add => {
                let other = other_value.expect("ADD requires two operands");
                if other.type_.name() == "String" {
                    // Number + String: convert number to string, concatenate.
                    // String constant pool management is handled at the VM level.
                    Value::new(other.value, Box::new(StringType))
                } else {
                    Value::new(this_value.value + other.value, Box::new(NumberType))
                }
            }
            Operation::Sub => {
                let other = other_value.expect("SUB requires two operands");
                Value::new(this_value.value - other.value, Box::new(NumberType))
            }
            Operation::Mult => {
                let other = other_value.expect("MULT requires two operands");
                Value::new(this_value.value * other.value, Box::new(NumberType))
            }
            Operation::Div => {
                let other = other_value.expect("DIV requires two operands");
                if other.value == 0 {
                    Value::new(0, Box::new(NumberType))
                } else {
                    Value::new(this_value.value / other.value, Box::new(NumberType))
                }
            }
            Operation::Mod => {
                let other = other_value.expect("MOD requires two operands");
                if other.value == 0 {
                    Value::new(0, Box::new(NumberType))
                } else {
                    Value::new(this_value.value % other.value, Box::new(NumberType))
                }
            }
            Operation::Not => {
                let result = if this_value.value == 0 { 1 } else { 0 };
                Value::new(result, Box::new(BooleanType))
            }
            Operation::Eql => {
                let other = other_value.expect("EQL requires two operands");
                let result = this_value.value == other.value;
                Value::new(i32::from(result), Box::new(BooleanType))
            }
            Operation::Gt => {
                let other = other_value.expect("GT requires two operands");
                let result = this_value.value > other.value;
                Value::new(i32::from(result), Box::new(BooleanType))
            }
            Operation::Lt => {
                let other = other_value.expect("LT requires two operands");
                let result = this_value.value < other.value;
                Value::new(i32::from(result), Box::new(BooleanType))
            }
            _ => panic!("Operation {:?} not supported on Number", op),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(NumberType)
    }
}
