use gs_core::core::exception::GvmExceptionHandler;
use gs_core::core::types::{TypeContext, Value};

use super::types::number::NumberType;
use super::types::object_type::ObjectType;
use super::types::string_type::StringType;
use super::GvmPlainObject;

pub struct GscriptExceptionHandler;

impl GvmExceptionHandler for GscriptExceptionHandler {
    fn convert_message(
        &self,
        message: &str,
        context: &mut dyn TypeContext,
        line: i32,
        location: i32,
    ) -> Value {
        let index = context.add_string(message);
        let exception_message = Value::new(index, Box::new(StringType));

        let mut exception_object = GvmPlainObject::new();
        exception_object.set_value_direct("message", exception_message);
        exception_object.set_value_direct("line", Value::new(line, Box::new(NumberType)));
        exception_object.set_value_direct(
            "location",
            Value::new(location, Box::new(StringType)),
        );
        let id = context.heap_add_object_box(Box::new(exception_object));
        Value::new(id, Box::new(ObjectType))
    }

    fn convert_value(
        &self,
        value: &Value,
        context: &mut dyn TypeContext,
        line: i32,
        location: i32,
    ) -> Value {
        let mut exception_object = GvmPlainObject::new();
        if value.type_.name() == "String" {
            exception_object.set_value_direct("message", value.clone());
        } else {
            exception_object.set_value_direct("exception", value.clone());
        }
        exception_object.set_value_direct("line", Value::new(line, Box::new(NumberType)));
        exception_object.set_value_direct(
            "location",
            Value::new(location, Box::new(StringType)),
        );
        let id = context.heap_add_object_box(Box::new(exception_object));
        Value::new(id, Box::new(ObjectType))
    }
}
