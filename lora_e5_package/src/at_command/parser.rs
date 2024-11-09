use heapless::{self, Vec}; // fixed capacity `std::Vec`
use defmt::unwrap;

use crate::at_command::command_set::{
    AtCmdStr,
    AtCommandSet,
};
use bm_network::{
    NetworkId, 
    bm_network_packet::bm_network_packet::BmNetworkPacketPayload
};

const BACKSPACE: u8 = 0x7F; // DEL sent when you hit backspace
const CARRIAGE_RETURN: u8 = 0x0D;

#[derive(Clone, PartialEq)]
pub struct CommandParser {
    // Buffer for incoming at commands
    cmd_buffer: AtCmdStr,
    // Buffer to store command arguments
    argument_buffer: AtCmdStr,
}

impl CommandParser {
    // Constructor which adds all supported AT commands
    pub fn new() -> CommandParser {
        CommandParser {
            cmd_buffer: AtCmdStr::new(),
            argument_buffer: AtCmdStr::new(),
        }        
    } 

    // Function to process 1 char at a time. Add them to internal buffers and decode AT commands.
    //
    // Returns tuple of: (decoded at command enum, t/f print help)
    pub fn handle_rx_char(&mut self, in_char: u8) -> Option<(AtCommandSet,bool)> {
        let mut command_accepted = AtCommandSet::Unknown;
        let mut print_help = false;
        
        // If enter character is received, handle command
        if in_char == CARRIAGE_RETURN {
            if self.cmd_buffer.len() == 0 {
                //defmt::info!("new line");
                return Some((AtCommandSet::NewLine, false));
            }
            else if self.cmd_buffer.len() < 2 {
                //defmt::info!("Command too short");

                // Clear command buffer
                self.cmd_buffer.clear();
                return None;
            }

            // Turn into slice
            let command_str = self.cmd_buffer.as_str();
            //defmt::info!("Command received: {}", command_str);

            // Check for AT at start
            if !command_str.starts_with("AT") {
                defmt::info!("Missing AT at start of string.");

                // Clear command buffer
                self.cmd_buffer.clear();

                return None;
            }
            
            // Handle Help Commands
            if command_str.contains("?") {
                // AT? is special case
                if command_str == "AT?" {
                    command_accepted = AtCommandSet::AtList;
                }
                else {
                    let truncated_str = &command_str[0..command_str.len() - 1];
                    if let Some(found_cmd) = AtCommandSet::match_command(truncated_str) {
                        defmt::info!("Command accepted: str={}, enum={}", command_str, found_cmd);
                        // Return matched help command
                        print_help = true;
                        command_accepted = found_cmd;
                    }
                }                
            }
            // Handle write commands
            else if command_str.contains("=") {
                let split_str: Vec<&str, 2> = command_str.split("=").collect();
                if let Some(found_cmd) = AtCommandSet::match_command(split_str[0]) {
                    if AtCommandSet::allow_write(found_cmd) {
                        // Grab str after command
                        if split_str.len() > 1 {
                            defmt::info!("arguments: {}", split_str[1]);
                            // Store arguments
                            self.argument_buffer.clear();
                            unwrap!(self.argument_buffer.push_str(split_str[1]));
                            
                            command_accepted = found_cmd;
                        }                        
                    }
                }
            }
            // Handle normal command
            else if let Some(found_cmd) = AtCommandSet::match_command(command_str) {
                defmt::info!("Command accepted: str={}, enum={}", command_str, found_cmd);
                // Return matched command
                command_accepted = found_cmd;                
            }

            // Clear command buffer
            self.cmd_buffer.clear();
            
            return Some((command_accepted, print_help))
        }
        else if in_char == BACKSPACE {
            if self.cmd_buffer.len() > 0 {
                self.cmd_buffer.pop().unwrap();                
            }
        }
        else {      
            // Add char to in buffer
            self.cmd_buffer.push(char::try_from(in_char).unwrap()).unwrap();           
        }

        None
    }

    pub fn get_cmd_arg(&mut self) -> AtCmdStr {
        self.argument_buffer.clone()
    }

    pub fn get_cmd_arg_as_u32(&mut self) -> Option<u32> {
        if self.argument_buffer.len() > 0 {
            let value: u32 = self.argument_buffer.parse().unwrap();
            return Some(value)
        }
        None
    }

    //-----------------------------------------------------------
    // Private functions
    //-----------------------------------------------------------    

}

pub type MessageTuple = (NetworkId, bool, u8, BmNetworkPacketPayload);

// Function to parse AT Cmd string into tuple of types used for packet.
pub fn cmd_arg_into_msg(argument_buffer: AtCmdStr) -> Option<MessageTuple> {
    // Expected format in the argument buffer: "dest,ack,ttl,ascii payload"
    let args: Vec<&str, 5> = argument_buffer.split(',').collect();
    
    if args.len() == 4 {
        // Create 3 types expected in the return
        let network_id = Some(args[0].parse().unwrap());
        let ack_required = args[1] == "true";
        let ttl = args[2].parse().unwrap();
        let mut payload: BmNetworkPacketPayload = Vec::new();
        unwrap!(payload.extend_from_slice(args[3].as_bytes()));
        // Combine all types into a tuple
        return Some((network_id, ack_required, ttl, payload))
    }
    else {
        defmt::error!("cmd_arg_into_msg: invalid args len={}", args.len());
    }
    None
}