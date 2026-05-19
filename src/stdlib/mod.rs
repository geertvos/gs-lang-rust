pub mod io;
pub mod net;
pub mod system;

use gs_core::bridge::registry::NativeRegistry;
use std::sync::Arc;

pub fn register_all(registry: &mut NativeRegistry) {
    registry.register(Arc::new(system::runtime::RuntimeModule));
    registry.register(Arc::new(net::server_socket::ServerSocketModule));
    registry.register(Arc::new(net::socket::SocketModule));
    registry.register(Arc::new(net::stream::TcpStreamModule));
    registry.register(Arc::new(net::stream::OutputStreamModule));
    registry.register(Arc::new(net::stream::InputStreamModule));
    registry.register(Arc::new(io::buffered_reader::BufferedReaderModule));
    registry.register(Arc::new(io::file::FileModule));
    registry.register(Arc::new(system::time::TimeModule));
    registry.register(Arc::new(system::env::EnvironmentModule));
}
