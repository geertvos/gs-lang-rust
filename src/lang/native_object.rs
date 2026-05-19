use std::any::Any;
use std::collections::HashMap;

use gs_core::bridge::NativeMethodWrapper;
use gs_core::bridge::native_module::{NativeInstance, NativeValue, MethodDescriptor};
use gs_core::core::object::GvmObject;
use gs_core::core::types::{FunctionType, TypeContext, Value};

pub struct NativeInstanceObject {
    instance: Box<dyn NativeInstance>,
    methods: HashMap<String, Value>,
}

impl std::fmt::Debug for NativeInstanceObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeInstanceObject({})", self.instance.type_name())
    }
}

impl Clone for NativeInstanceObject {
    fn clone(&self) -> Self {
        NativeInstanceObject {
            instance: self.instance.clone_instance(),
            methods: self.methods.clone(),
        }
    }
}

impl NativeInstanceObject {
    pub fn new(instance: Box<dyn NativeInstance>) -> Self {
        NativeInstanceObject {
            instance,
            methods: HashMap::new(),
        }
    }

    pub fn type_name(&self) -> &str {
        self.instance.type_name()
    }

    pub fn instance(&self) -> &dyn NativeInstance {
        self.instance.as_ref()
    }

    pub fn instance_methods(&self) -> Vec<MethodDescriptor> {
        self.instance.instance_methods()
    }

    pub fn methods_iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.methods.iter()
    }

    pub fn register_methods(&mut self, obj_ref: i32, context: &mut dyn TypeContext) {
        let methods: Vec<(String, i32)> = self
            .instance_methods()
            .iter()
            .map(|m| (m.name.clone(), m.arg_count))
            .collect();
        for (name, arg_count) in methods {
            let wrapper = NativeInstanceMethodWrapper::new(obj_ref, name.clone(), arg_count);
            let func_idx = context.generate_native_method_function(Box::new(wrapper), arg_count);
            self.methods
                .insert(name, Value::new(func_idx, Box::new(FunctionType)));
        }
    }
}

impl GvmObject for NativeInstanceObject {
    fn set_value(&mut self, id: &str, v: Value) {
        self.methods.insert(id.to_string(), v);
    }

    fn get_value(&self, id: &str) -> Option<Value> {
        self.methods.get(id).cloned()
    }

    fn has_value(&self, id: &str) -> bool {
        self.methods.contains_key(id)
    }

    fn get_values(&self) -> Vec<Value> {
        self.methods.values().cloned().collect()
    }

    fn get_keys(&self) -> Vec<String> {
        self.methods.keys().cloned().collect()
    }

    fn pre_destroy(&self) {
        self.instance.destroy();
    }

    fn clone_object(&self) -> Box<dyn GvmObject> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct NativeInstanceMethodWrapper {
    obj_ref: i32,
    method_name: String,
    arg_count: i32,
}

impl NativeInstanceMethodWrapper {
    pub fn new(obj_ref: i32, method_name: String, arg_count: i32) -> Self {
        NativeInstanceMethodWrapper {
            obj_ref,
            method_name,
            arg_count,
        }
    }
}

impl NativeMethodWrapper for NativeInstanceMethodWrapper {
    fn invoke(&self, arguments: Vec<Value>, context: &mut dyn TypeContext) -> Result<Value, String> {
        let mut args = arguments;
        args.reverse();

        let native_args = marshal_args_to_native(&args, context);

        let instance_clone = context
            .heap_get_object_any(self.obj_ref)
            .and_then(|a| a.downcast_ref::<NativeInstanceObject>())
            .map(|obj| obj.instance().clone_instance());

        let result = match instance_clone {
            Some(inst) => inst.call_method(&self.method_name, native_args),
            None => Err(gs_core::bridge::native_module::NativeError::new(format!(
                "Object at ref {} is not a NativeInstanceObject",
                self.obj_ref
            ))),
        };

        match result {
            Ok(native_val) => Ok(marshal_native_to_value(native_val, context)),
            Err(e) => Err(format!("{}", e)),
        }
    }

    fn argument_count(&self) -> i32 {
        self.arg_count
    }
}

pub fn marshal_value_to_native(value: &Value, context: &dyn TypeContext) -> NativeValue {
    match value.type_.name() {
        "String" => {
            let s = context.get_string(value.value).unwrap_or("").to_string();
            NativeValue::String(s)
        }
        "Number" => NativeValue::Number(value.value),
        "Boolean" => NativeValue::Boolean(value.value > 0),
        "Undefined" => NativeValue::Undefined,
        "NativeObject" => {
            let obj = context
                .heap_get_object_any(value.value)
                .and_then(|a| a.downcast_ref::<NativeInstanceObject>());
            match obj {
                Some(native_obj) => {
                    if let Some(ba) = native_obj.instance().as_any().downcast_ref::<ByteArrayInstance>() {
                        NativeValue::Bytes(ba.get_bytes().to_vec())
                    } else {
                        NativeValue::Instance(native_obj.instance().clone_instance())
                    }
                }
                None => NativeValue::Undefined,
            }
        }
        _ => NativeValue::Number(value.value),
    }
}

fn marshal_args_to_native(args: &[Value], context: &dyn TypeContext) -> Vec<NativeValue> {
    args.iter()
        .map(|v| marshal_value_to_native(v, context))
        .collect()
}

pub fn marshal_native_to_value(native_val: NativeValue, context: &mut dyn TypeContext) -> Value {
    use crate::lang::types::native_object_type::NativeObjectType;
    use crate::lang::types::string_type::StringType;

    match native_val {
        NativeValue::Instance(inst) => {
            let mut obj = NativeInstanceObject::new(inst);
            let new_ref = context.heap_add_object_box(Box::new(obj.clone()));
            obj.register_methods(new_ref, context);
            for (name, value) in obj.methods_iter() {
                context.heap_set_object_value(new_ref, name, value.clone());
            }
            Value::new(new_ref, Box::new(NativeObjectType))
        }
        NativeValue::String(s) => {
            let idx = context.add_string(&s);
            Value::new(idx, Box::new(StringType))
        }
        NativeValue::Number(n) => {
            Value::new(n, Box::new(crate::lang::types::number::NumberType))
        }
        NativeValue::Boolean(b) => {
            Value::new(if b { 1 } else { 0 }, Box::new(gs_core::core::types::BooleanType))
        }
        NativeValue::Bytes(data) => {
            let mut obj = NativeInstanceObject::new(Box::new(ByteArrayInstance { data }));
            let new_ref = context.heap_add_object_box(Box::new(obj.clone()));
            obj.register_methods(new_ref, context);
            for (name, value) in obj.methods_iter() {
                context.heap_set_object_value(new_ref, name, value.clone());
            }
            Value::new(new_ref, Box::new(NativeObjectType))
        }
        NativeValue::Undefined => Value::undefined(),
    }
}

struct ByteArrayInstance {
    data: Vec<u8>,
}

impl NativeInstance for ByteArrayInstance {
    fn type_name(&self) -> &str {
        "ByteArray"
    }

    fn instance_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }

    fn call_method(&self, method: &str, _args: Vec<NativeValue>) -> Result<NativeValue, gs_core::bridge::native_module::NativeError> {
        Err(gs_core::bridge::native_module::NativeError::new(format!(
            "Unknown method {} on ByteArray",
            method
        )))
    }

    fn destroy(&self) {}

    fn clone_instance(&self) -> Box<dyn NativeInstance> {
        Box::new(ByteArrayInstance {
            data: self.data.clone(),
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ByteArrayInstance {
    pub fn get_bytes(&self) -> &[u8] {
        &self.data
    }
}
