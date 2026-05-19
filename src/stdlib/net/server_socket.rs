use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeInstance, NativeModule, NativeResult, NativeValue,
};

use super::stream::TcpStreamInstance;

pub struct ServerSocketModule;

impl NativeModule for ServerSocketModule {
    fn class_name(&self) -> &str {
        "gs.net.ServerSocket"
    }

    fn constructor(&self, args: Vec<NativeValue>) -> NativeResult {
        let port = match args.first() {
            Some(NativeValue::Number(p)) => *p as u16,
            _ => 8080,
        };
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr)
            .map_err(|e| NativeError::new(format!("Failed to bind to {}: {}", addr, e)))?;
        Ok(NativeValue::Instance(Box::new(ServerSocketInstance {
            listener: Arc::new(Mutex::new(listener)),
        })))
    }

    fn call_static(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown static method {} on gs.net.ServerSocket",
            method
        )))
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }
}

pub struct ServerSocketInstance {
    listener: Arc<Mutex<TcpListener>>,
}

impl NativeInstance for ServerSocketInstance {
    fn type_name(&self) -> &str {
        "ServerSocket"
    }

    fn instance_methods(&self) -> Vec<MethodDescriptor> {
        vec![
            MethodDescriptor::new("accept", 0),
            MethodDescriptor::new("close", 0),
        ]
    }

    fn call_method(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        match method {
            "accept" => {
                let l = self.listener.lock().unwrap();
                match l.accept() {
                    Ok((stream, _addr)) => Ok(NativeValue::Instance(Box::new(
                        TcpStreamInstance::new(stream),
                    ))),
                    Err(e) => Err(NativeError::new(format!("accept failed: {}", e))),
                }
            }
            "close" => {
                // TcpListener doesn't have an explicit close; dropping it closes.
                // We could drop the Arc here, but that's complex. Just return.
                Ok(NativeValue::Undefined)
            }
            _ => Err(NativeError::new(format!(
                "Unknown method {} on ServerSocket",
                method
            ))),
        }
    }

    fn destroy(&self) {}

    fn clone_instance(&self) -> Box<dyn NativeInstance> {
        Box::new(ServerSocketInstance {
            listener: self.listener.clone(),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
