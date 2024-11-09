use core::fmt::Write;
use heapless::String;
use bm_network::{bm_network_node::bm_network_node::BmNodeEntry, bm_network_packet::bm_network_packet::BmNetworkPacket};
use defmt::unwrap;

use crate::at_command::command_set::{
    AtCmdStr,
    AtCommandSet,
};

#[derive(Clone, PartialEq)]
pub struct ResponseGenerator {
    // Buffer for at command responses
    resp_buffer: AtCmdStr,
}

impl ResponseGenerator {
    pub fn new() -> ResponseGenerator {
        ResponseGenerator {
            resp_buffer: AtCmdStr::new(),
        }
    }

    /// Function to format a response string with the pre-configured 
    /// response to 'resp_enum' concattenated with 'resp_val'.
    pub fn fmt_resp_str_as_str_slice(&mut self, resp_enum: AtCommandSet, resp_val: &str) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.push_str("\n\r"));

        // Add pre canned response str
        if let Some(resp) = AtCommandSet::get_response(resp_enum) {
            // Stupid String library has a runtime error at pushing a str len of 1
            if resp.len() == 1 {
                let ch = resp.chars().next().unwrap();
                unwrap!(self.resp_buffer.push(ch));
            }
            else if resp.len() > 2 {
                unwrap!(self.resp_buffer.push_str(resp));
            }
        }
        
        // Add response value to buffer
        unwrap!(self.resp_buffer.push_str(resp_val));

        // Add generic OK and >
        unwrap!(self.resp_buffer.push_str("\n\rOK\n\r>"));

        self.resp_buffer.as_bytes()
    }

    pub fn fmt_resp_uint_as_str_slice(&mut self, resp_enum: AtCommandSet, resp_val: u32) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.push_str("\n\r"));

        // Add pre canned response str
        if let Some(resp) = AtCommandSet::get_response(resp_enum) {
            // Stupid String library has a runtime error at pushing a str len of 1
            if resp.len() == 1 {
                let ch = resp.chars().next().unwrap();
                unwrap!(self.resp_buffer.push(ch));
            }
            else if resp.len() > 2 {
                unwrap!(self.resp_buffer.push_str(resp));
            }
        }
        
        // Add response value to buffer
        let resp_val_str: String<10> = String::try_from(resp_val).unwrap();
        unwrap!(self.resp_buffer.push_str(resp_val_str.trim()));

        // Add generic OK and >
        unwrap!(self.resp_buffer.push_str("\n\rOK\n\r>"));

        self.resp_buffer.as_bytes()
    }

    pub fn fmt_resp_packet_as_str_slice(&mut self, in_msg: &mut BmNetworkPacket) -> &[u8] {
        self.resp_buffer.clear();
        // Print out custom formatted packet to resp_buffer
        // +<originator>,<num hops>,<rssi>,<length>,<payload(optional)>
        // OK
        // >
        let payload_len = in_msg.get_payload_len();

        self.resp_buffer.write_fmt(
            format_args!("\n\r+{},{},{},{}", 
                in_msg.get_originator().unwrap_or(0),
                in_msg.get_hop_count(),
                in_msg.rx_rssi,
                payload_len,
            )
        ).unwrap();

        // If a payload is available append to resp buffer
        if let Some(payload) = in_msg.get_payload() {
            unwrap!(self.resp_buffer.push_str(","));
            for &ch in payload {
                unwrap!(self.resp_buffer.push(char::try_from(ch).unwrap()));
            }
        }

        // Add generic OK and >
        unwrap!(self.resp_buffer.push_str("\n\rOK\n\r>"));

        self.resp_buffer.as_bytes()
    }

    pub fn fmt_resp_node_as_str_slice(&mut self, node_data: &BmNodeEntry) -> &[u8] {
        // Write struct to String. Formatter is implemented in node file
        self.resp_buffer.write_fmt(format_args!("\n\r{}", node_data)).unwrap();
        self.resp_buffer.as_bytes()
    }

    pub fn get_help_str(&mut self, resp_enum: AtCommandSet) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.push_str("\n\r"));

        // Add pre canned help str
        if let Some(resp) = AtCommandSet::get_help(resp_enum) {
            if resp.len() > 2 {
                unwrap!(self.resp_buffer.push_str(resp));
            }
        }

        unwrap!(self.resp_buffer.push_str("\n\r>"));
        
        self.resp_buffer.as_bytes()
    }

    pub fn get_available_cmds(&mut self) -> &[u8] {
        self.resp_buffer.clear();

        // Get list of commands from at_command_handler
        AtCommandSet::get_available_cmds(&mut self.resp_buffer);

        self.resp_buffer.as_bytes()
    }

    pub fn as_string_slice(&mut self) -> &[u8] {
        self.resp_buffer.as_bytes()
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

}