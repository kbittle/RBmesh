#![no_std]

// Device network identifier
pub type NetworkId = Option<u32>;

// Radio received signal strength
pub type RssiType = i16;

// Date Time timestamp
pub type TimeType = i64;

#[derive(Default, PartialEq)]
pub enum BmError {
    #[default]
    None,
    Busy,
    QueueFull,
}

pub mod bm_network_configs;
pub mod bm_network_engine;
pub mod bm_network_routing_table;
pub mod bm_network_node;
pub mod bm_network_packet;

#[cfg(test)]
pub mod test;
