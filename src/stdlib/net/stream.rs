use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeInstance, NativeModule, NativeResult, NativeValue,
};

// -- TcpStream module (not directly constructable, returned by ServerSocket.accept) --

pub struct TcpStreamModule;

impl NativeModule for TcpStreamModule {
    fn class_name(&self) -> &str {
        "gs.net.TcpStream"
    }

    fn constructor(&self, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(
            "gs.net.TcpStream cannot be constructed directly; use ServerSocket.accept()",
        ))
    }

    fn call_static(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown static method {} on gs.net.TcpStream",
            method
        )))
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }
}

pub struct TcpStreamInstance {
    stream: Arc<Mutex<TcpStream>>,
}

impl TcpStreamInstance {
    pub fn new(stream: TcpStream) -> Self {
        TcpStreamInstance {
            stream: Arc::new(Mutex::new(stream)),
        }
    }
}

impl NativeInstance for TcpStreamInstance {
    fn type_name(&self) -> &str {
        "TcpStream"
    }

    fn instance_methods(&self) -> Vec<MethodDescriptor> {
        vec![
            MethodDescriptor::new("getInputStream", 0),
            MethodDescriptor::new("getOutputStream", 0),
            MethodDescriptor::new("close", 0),
        ]
    }

    fn call_method(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        match method {
            "getInputStream" => Ok(NativeValue::Instance(Box::new(InputStreamInstance {
                stream: self.stream.clone(),
            }))),
            "getOutputStream" => Ok(NativeValue::Instance(Box::new(OutputStreamInstance {
                stream: self.stream.clone(),
            }))),
            "close" => {
                if let Ok(s) = self.stream.lock() {
                    let _ = s.shutdown(std::net::Shutdown::Both);
                }
                Ok(NativeValue::Undefined)
            }
            _ => Err(NativeError::new(format!(
                "Unknown method {} on TcpStream",
                method
            ))),
        }
    }

    fn destroy(&self) {}

    fn clone_instance(&self) -> Box<dyn NativeInstance> {
        Box::new(TcpStreamInstance {
            stream: self.stream.clone(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// -- InputStream module --

pub struct InputStreamModule;

impl NativeModule for InputStreamModule {
    fn class_name(&self) -> &str {
        "gs.net.InputStream"
    }

    fn constructor(&self, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(
            "gs.net.InputStream cannot be constructed directly",
        ))
    }

    fn call_static(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown static method {} on gs.net.InputStream",
            method
        )))
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }
}

pub struct InputStreamInstance {
    pub stream: Arc<Mutex<TcpStream>>,
}

impl NativeInstance for InputStreamInstance {
    fn type_name(&self) -> &str {
        "InputStream"
    }

    fn instance_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }

    fn call_method(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown method {} on InputStream",
            method
        )))
    }

    fn destroy(&self) {}

    fn clone_instance(&self) -> Box<dyn NativeInstance> {
        Box::new(InputStreamInstance {
            stream: self.stream.clone(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// -- OutputStream module --

pub struct OutputStreamModule;

impl NativeModule for OutputStreamModule {
    fn class_name(&self) -> &str {
        "gs.net.OutputStream"
    }

    fn constructor(&self, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(
            "gs.net.OutputStream cannot be constructed directly",
        ))
    }

    fn call_static(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown static method {} on gs.net.OutputStream",
            method
        )))
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }
}

pub struct OutputStreamInstance {
    stream: Arc<Mutex<TcpStream>>,
}

impl NativeInstance for OutputStreamInstance {
    fn type_name(&self) -> &str {
        "OutputStream"
    }

    fn instance_methods(&self) -> Vec<MethodDescriptor> {
        vec![
            MethodDescriptor::new("write", 1),
            MethodDescriptor::new("flush", 0),
            MethodDescriptor::new("close", 0),
        ]
    }

    fn call_method(&self, method: &str, args: Vec<NativeValue>) -> NativeResult {
        match method {
            "write" => {
                let data = match args.first() {
                    Some(NativeValue::Bytes(b)) => b.clone(),
                    Some(NativeValue::String(s)) => s.as_bytes().to_vec(),
                    _ => {
                        return Err(NativeError::new("write: expected Bytes or String argument"))
                    }
                };
                let mut s = self
                    .stream
                    .lock()
                    .map_err(|_| NativeError::new("Failed to lock stream"))?;
                s.write_all(&data)
                    .map_err(|e| NativeError::new(format!("write failed: {}", e)))?;
                Ok(NativeValue::Undefined)
            }
            "flush" => {
                if let Ok(mut s) = self.stream.lock() {
                    let _ = s.flush();
                }
                Ok(NativeValue::Undefined)
            }
            "close" => {
                if let Ok(s) = self.stream.lock() {
                    let _ = s.shutdown(std::net::Shutdown::Write);
                }
                Ok(NativeValue::Undefined)
            }
            _ => Err(NativeError::new(format!(
                "Unknown method {} on OutputStream",
                method
            ))),
        }
    }

    fn destroy(&self) {}

    fn clone_instance(&self) -> Box<dyn NativeInstance> {
        Box::new(OutputStreamInstance {
            stream: self.stream.clone(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
