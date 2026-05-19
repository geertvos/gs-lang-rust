use std::fs;
use std::io::Write;

use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeInstance, NativeModule, NativeResult, NativeValue,
};

pub struct FileModule;

impl NativeModule for FileModule {
    fn class_name(&self) -> &str {
        "gs.io.File"
    }

    fn constructor(&self, args: Vec<NativeValue>) -> NativeResult {
        match args.into_iter().next() {
            Some(NativeValue::String(path)) => {
                Ok(NativeValue::Instance(Box::new(FileInstance { path })))
            }
            _ => Err(NativeError::new("File: requires a String path argument")),
        }
    }

    fn call_static(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown static method {} on gs.io.File",
            method
        )))
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }
}

struct FileInstance {
    path: String,
}

impl NativeInstance for FileInstance {
    fn type_name(&self) -> &str {
        "File"
    }

    fn instance_methods(&self) -> Vec<MethodDescriptor> {
        vec![
            MethodDescriptor::new("read", 0),
            MethodDescriptor::new("write", 1),
            MethodDescriptor::new("append", 1),
            MethodDescriptor::new("exists", 0),
            MethodDescriptor::new("delete", 0),
        ]
    }

    fn call_method(&self, method: &str, args: Vec<NativeValue>) -> NativeResult {
        match method {
            "read" => {
                let contents = fs::read_to_string(&self.path)
                    .map_err(|e| NativeError::new(format!("read failed: {}", e)))?;
                Ok(NativeValue::String(contents))
            }
            "write" => {
                let data = extract_string_arg(&args, "write")?;
                fs::write(&self.path, &data)
                    .map_err(|e| NativeError::new(format!("write failed: {}", e)))?;
                Ok(NativeValue::Undefined)
            }
            "append" => {
                let data = extract_string_arg(&args, "append")?;
                let mut file = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&self.path)
                    .map_err(|e| NativeError::new(format!("append open failed: {}", e)))?;
                file.write_all(data.as_bytes())
                    .map_err(|e| NativeError::new(format!("append write failed: {}", e)))?;
                Ok(NativeValue::Undefined)
            }
            "exists" => Ok(NativeValue::Boolean(
                std::path::Path::new(&self.path).exists(),
            )),
            "delete" => {
                let result = fs::remove_file(&self.path).is_ok();
                Ok(NativeValue::Boolean(result))
            }
            _ => Err(NativeError::new(format!(
                "Unknown method {} on File",
                method
            ))),
        }
    }

    fn destroy(&self) {}

    fn clone_instance(&self) -> Box<dyn NativeInstance> {
        Box::new(FileInstance {
            path: self.path.clone(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn extract_string_arg(args: &[NativeValue], method_name: &str) -> Result<String, NativeError> {
    match args.first() {
        Some(NativeValue::String(s)) => Ok(s.clone()),
        Some(NativeValue::Bytes(b)) => String::from_utf8(b.clone())
            .map_err(|e| NativeError::new(format!("{}: invalid UTF-8: {}", method_name, e))),
        _ => Err(NativeError::new(format!(
            "{}: expected String argument",
            method_name
        ))),
    }
}
