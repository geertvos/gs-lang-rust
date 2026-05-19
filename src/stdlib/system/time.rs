use std::time::{SystemTime, UNIX_EPOCH};

use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeModule, NativeResult, NativeValue,
};

pub struct TimeModule;

impl NativeModule for TimeModule {
    fn class_name(&self) -> &str {
        "gs.system.Time"
    }

    fn constructor(&self, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new("gs.system.Time is static-only"))
    }

    fn call_static(&self, method: &str, args: Vec<NativeValue>) -> NativeResult {
        match method {
            "now" => {
                let millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i32;
                Ok(NativeValue::Number(millis))
            }
            "sleep" => {
                let ms = match args.first() {
                    Some(NativeValue::Number(n)) => *n as u64,
                    _ => 0,
                };
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(NativeValue::Undefined)
            }
            _ => Err(NativeError::new(format!(
                "Unknown static method {} on gs.system.Time",
                method
            ))),
        }
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![
            MethodDescriptor::new("now", 0),
            MethodDescriptor::new("sleep", 1),
        ]
    }
}
