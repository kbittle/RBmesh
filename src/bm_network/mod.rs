// Device network identifier
pub type NetworkId = Option<u32>;

// Radio received signal strength
pub type RssiType = i16;

// Date Time timestamp
pub type TimeType = i64;

pub enum BmErrors {
    NoPacket,
    QueueFull,
    GenericError,
}

pub mod bm_network_configs;
pub mod bm_network_engine;
pub mod bm_network_stack;
pub mod bm_network_node;
pub mod bm_network_packet;
