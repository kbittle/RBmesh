use stm32wlxx_hal::Ratio;
use super::bm_network_stack;

const RADIO_MAX_BUFF_SIZE: usize = 255;

#[derive(Debug, Clone)]
pub struct RadioRxBuffer {
    pub length: u8,
    pub buffer: [u8; RADIO_MAX_BUFF_SIZE],
    pub rssi: bm_network_stack::RssiType,
}

impl RadioRxBuffer {
    pub fn new() -> RadioRxBuffer {
        RadioRxBuffer {
            length: 0,
            buffer: [0; RADIO_MAX_BUFF_SIZE],
            rssi: 0,
        }
    }

    pub const fn with_len(self, length:u8) -> Self {
        RadioRxBuffer {
            length: length,
            buffer: self.buffer,
            rssi: self.rssi,
        }
    }

    pub const fn with_rssi(self, rssi:i16) -> Self {
        RadioRxBuffer {
            length: self.length,
            buffer: self.buffer,
            rssi: rssi,
        }
    }
}

pub mod bm_radio_control;