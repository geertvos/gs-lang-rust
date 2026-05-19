use std::io::{BufRead, BufReader, Read};
use std::sync::{Arc, Mutex};

use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeInstance, NativeModule, NativeResult, NativeValue,
};

use crate::stdlib::net::stream::InputStreamInstance;

pub struct BufferedReaderModule;

impl NativeModule for BufferedReaderModule {
    fn class_name(&self) -> &str {
        "gs.io.BufferedReader"
    }

    fn constructor(&self, args: Vec<NativeValue>) -> NativeResult {
        match args.into_iter().next() {
            Some(NativeValue::Instance(inst)) => {
                let input: &InputStreamInstance = inst
                    .as_any()
                    .downcast_ref()
                    .ok_or_else(|| {
                        NativeError::new("BufferedReader: argument is not an InputStream")
                    })?;
                let cloned = input
                    .stream
                    .lock()
                    .unwrap()
                    .try_clone()
                    .map_err(|e| NativeError::new(format!("Failed to clone stream: {}", e)))?;
                let reader = BufReader::new(Box::new(cloned) as Box<dyn Read + Send>);
                Ok(NativeValue::Instance(Box::new(BufferedReaderInstance {
                    reader: Arc::new(Mutex::new(reader)),
                })))
            }
            _ => Err(NativeError::new(
                "BufferedReader: requires an InputStream argument",
            )),
        }
    }

    fn call_static(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown static method {} on gs.io.BufferedReader",
            method
        )))
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }
}

struct BufferedReaderInstance {
    reader: Arc<Mutex<BufReader<Box<dyn Read + Send>>>>,
}

impl NativeInstance for BufferedReaderInstance {
    fn type_name(&self) -> &str {
        "BufferedReader"
    }

    fn instance_methods(&self) -> Vec<MethodDescriptor> {
        vec![MethodDescriptor::new("readLine", 0)]
    }

    fn call_method(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        match method {
            "readLine" => {
                let mut r = self.reader.lock().unwrap();
                let mut line = String::new();
                match r.read_line(&mut line) {
                    Ok(0) => Ok(NativeValue::Undefined),
                    Ok(_) => {
                        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
                        Ok(NativeValue::String(trimmed.to_string()))
                    }
                    Err(e) => Err(NativeError::new(format!("readLine failed: {}", e))),
                }
            }
            _ => Err(NativeError::new(format!(
                "Unknown method {} on BufferedReader",
                method
            ))),
        }
    }

    fn destroy(&self) {}

    fn clone_instance(&self) -> Box<dyn NativeInstance> {
        Box::new(BufferedReaderInstance {
            reader: self.reader.clone(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
