#![cfg_attr(not(test), no_std)]

// Device network identifier
pub type NetworkId = Option<u32>;

// Radio received signal strength
pub type RssiType = i16;

// Date Time timestamp
pub type TimeType = i64;

#[derive(Default, PartialEq, Debug)]
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

// Include stubs whenever building for host OS (Linux/WSL) so integration tests link cleanly:
#[cfg(not(target_os = "none"))]
mod test_stubs {
    // Critical Section stubs
    #[no_mangle]
    unsafe fn _critical_section_1_0_acquire() -> u8 {
        0
    }

    #[no_mangle]
    unsafe fn _critical_section_1_0_release(_restore_state: u8) {}

    // defmt logger stubs for host test execution
    #[no_mangle]
    fn _defmt_acquire() {}

    #[no_mangle]
    fn _defmt_release() {}

    #[no_mangle]
    fn _defmt_write(_bytes: &[u8]) {}
}
