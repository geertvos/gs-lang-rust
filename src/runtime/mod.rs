use std::sync::Arc;

use gs_core::bridge::NativeMethodWrapper;
use gs_core::bridge::native_module::NativeValue;
use gs_core::bridge::registry::NativeRegistry;
use gs_core::core::types::{TypeContext, Value};

use crate::lang::native_object::{marshal_native_to_value, marshal_value_to_native};

pub struct NativeStaticMethodWrapper {
    arg_count: i32,
    registry: Arc<NativeRegistry>,
}

impl NativeStaticMethodWrapper {
    pub fn new(arg_count: i32, registry: Arc<NativeRegistry>) -> Self {
        NativeStaticMethodWrapper { arg_count, registry }
    }
}

impl NativeMethodWrapper for NativeStaticMethodWrapper {
    fn invoke(&self, arguments: Vec<Value>, context: &mut dyn TypeContext) -> Result<Value, String> {
        let mut args = arguments;
        args.reverse();

        let class_name = context
            .get_string(args[0].value)
            .unwrap_or("")
            .to_string();
        let method_name = context
            .get_string(args[1].value)
            .unwrap_or("")
            .to_string();

        let native_args: Vec<NativeValue> = args[2..]
            .iter()
            .map(|v| marshal_value_to_native(v, context))
            .collect();

        let result = self
            .registry
            .dispatch(&class_name, &method_name, native_args);

        match result {
            Ok(native_val) => Ok(marshal_native_to_value(native_val, context)),
            Err(e) => Err(format!("{}", e)),
        }
    }

    fn argument_count(&self) -> i32 {
        self.arg_count
    }
}
