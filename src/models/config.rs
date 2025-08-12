/// Configuration options specific to the Dantes service.
#[derive(Clone)]
pub struct ServerConfig {
    /// Address of the ZeroMQ socket used for communication with workers.
    pub zmq_address: String,
}
