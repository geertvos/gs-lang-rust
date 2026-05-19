use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeModule, NativeResult, NativeValue,
};

pub struct EnvironmentModule;

impl NativeModule for EnvironmentModule {
    fn class_name(&self) -> &str {
        "gs.system.Environment"
    }

    fn constructor(&self, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new("gs.system.Environment is static-only"))
    }

    fn call_static(&self, method: &str, args: Vec<NativeValue>) -> NativeResult {
        match method {
            "get" => match args.first() {
                Some(NativeValue::String(key)) => match std::env::var(key) {
                    Ok(val) => Ok(NativeValue::String(val)),
                    Err(_) => Ok(NativeValue::Undefined),
                },
                _ => Err(NativeError::new("Environment.get: expected String key")),
            },
            "set" => {
                let key = match args.first() {
                    Some(NativeValue::String(k)) => k.clone(),
                    _ => return Err(NativeError::new("Environment.set: expected String key")),
                };
                let value = match args.get(1) {
                    Some(NativeValue::String(v)) => v.clone(),
                    _ => {
                        return Err(NativeError::new("Environment.set: expected String value"))
                    }
                };
                unsafe {
                    std::env::set_var(&key, &value);
                }
                Ok(NativeValue::Undefined)
            }
            _ => Err(NativeError::new(format!(
                "Unknown static method {} on gs.system.Environment",
                method
            ))),
        }
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![
            MethodDescriptor::new("get", 1),
            MethodDescriptor::new("set", 2),
        ]
    }
}
