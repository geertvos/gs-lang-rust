use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeModule, NativeResult, NativeValue,
};

pub struct RuntimeModule;

impl NativeModule for RuntimeModule {
    fn class_name(&self) -> &str {
        "gs.system.Runtime"
    }

    fn constructor(&self, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new("gs.system.Runtime is static-only"))
    }

    fn call_static(&self, method: &str, args: Vec<NativeValue>) -> NativeResult {
        match method {
            "print" => {
                if let Some(arg) = args.first() {
                    match arg {
                        NativeValue::String(s) => println!("{}", s),
                        NativeValue::Number(n) => println!("{}", n),
                        NativeValue::Boolean(b) => println!("{}", b),
                        NativeValue::Undefined => println!("undefined"),
                        _ => println!("{:?}", arg),
                    }
                }
                Ok(NativeValue::Undefined)
            }
            _ => Err(NativeError::new(format!(
                "Unknown static method {} on gs.system.Runtime",
                method
            ))),
        }
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![MethodDescriptor::new("print", 1)]
    }
}
