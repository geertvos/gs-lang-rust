use gs_core::core::types::{Operation, Type, TypeContext, Value};

#[derive(Debug, Clone)]
pub struct NativeObjectType;

impl Type for NativeObjectType {
    fn name(&self) -> &str {
        "NativeObject"
    }

    fn supports_operation(&self, op: Operation) -> bool {
        matches!(op, Operation::Get)
    }

    fn perform(
        &self,
        context: &mut dyn TypeContext,
        op: Operation,
        this_value: &Value,
        other_value: Option<&Value>,
    ) -> Value {
        match op {
            Operation::Get => {
                let field = other_value.expect("GET requires a field operand");
                let field_name = if field.type_.name() == "String" {
                    context.get_string(field.value).unwrap_or("").to_string()
                } else {
                    return Value::undefined();
                };
                context
                    .heap_get_object_value(this_value.value, &field_name)
                    .unwrap_or_else(Value::undefined)
            }
            _ => panic!("Operation {:?} not supported on NativeObject", op),
        }
    }

    fn is_instance(&self, other: &dyn Type) -> bool {
        other.name() == self.name()
    }

    fn clone_type(&self) -> Box<dyn Type> {
        Box::new(NativeObjectType)
    }
}
