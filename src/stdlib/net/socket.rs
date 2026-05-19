use std::net::TcpStream;

use gs_core::bridge::native_module::{
    MethodDescriptor, NativeError, NativeModule, NativeResult, NativeValue,
};

use super::stream::TcpStreamInstance;

pub struct SocketModule;

impl NativeModule for SocketModule {
    fn class_name(&self) -> &str {
        "gs.net.Socket"
    }

    fn constructor(&self, args: Vec<NativeValue>) -> NativeResult {
        let host = match args.first() {
            Some(NativeValue::String(h)) => h.clone(),
            _ => return Err(NativeError::new("Socket: first argument must be a host string")),
        };
        let port = match args.get(1) {
            Some(NativeValue::Number(p)) => *p as u16,
            _ => return Err(NativeError::new("Socket: second argument must be a port number")),
        };
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr)
            .map_err(|e| NativeError::new(format!("Failed to connect to {}: {}", addr, e)))?;
        Ok(NativeValue::Instance(Box::new(TcpStreamInstance::new(stream))))
    }

    fn call_static(&self, method: &str, _args: Vec<NativeValue>) -> NativeResult {
        Err(NativeError::new(format!(
            "Unknown static method {} on gs.net.Socket",
            method
        )))
    }

    fn static_methods(&self) -> Vec<MethodDescriptor> {
        vec![]
    }
}
