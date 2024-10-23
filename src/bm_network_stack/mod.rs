use stm32wlxx_hal::Ratio;

// Device network identifier
pub type NetworkId = Option<u32>;

// Radio received signal strength
pub type RssiType = i16;

pub enum BmErrors {
    NoPacket,
    QueueFull,
    GenericError,
}

pub mod bm_network_stack;
pub mod bm_network_node;
pub mod bm_network_packet;
