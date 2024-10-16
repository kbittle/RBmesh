use heapless::{self, Vec, String}; // fixed capacity `std::Vec`

#[derive(Default, Debug, Clone, PartialEq)]
pub struct AtCmdResp {
    resp_buffer: Vec<u8, 512>,
}

impl AtCmdResp {
    pub fn handle_command(&mut self, command: &mut Vec<u8, 512>) -> bool {

        let command_slice = command.as_slice();
        defmt::info!("Command: {}", command_slice);    
    
        match command_slice {
            b"AT" => {
                // Just showing a diff way to do this
                self.load_resp_buf("Ok");
                true            
            },
            b"AT+GMR" => self.handle_version(),
            b"AT+CSQ" => self.Handle_csq(),
            b"AT+ID?" => self.handle_get_net_id(),
            _ => { false },
        }
    }

    // Private functions
    fn load_resp_buf(&mut self, _resp:&str) {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        self.resp_buffer.extend_from_slice(_resp.as_bytes());
    }

    fn handle_version(&mut self) -> bool {
        // Example firmware version
        self.load_resp_buf("Version: 1.0.0");
        true
    }
    
    fn Handle_csq(&mut self) -> bool {
        // Example signal quality
        self.load_resp_buf("+CSQ: 15,99");
        true
    }
    
    fn handle_get_net_id(&mut self) -> bool {
        self.load_resp_buf("+ID=5");
        true
    }
}
