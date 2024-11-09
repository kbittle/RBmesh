use heapless::{self, Vec}; // fixed capacity `std::Vec`
use defmt::unwrap;

use crate::at_cmd_handler::at_cmd::{
    AtCmdStr,
    AtCommand,
};

const BACKSPACE: u8 = 0x7F; // DEL sent when you hit backspace
const CARRIAGE_RETURN: u8 = 0x0D;

#[derive(Clone, PartialEq)]
pub struct AtCmdResp {
    // Buffer for incoming at commands
    cmd_buffer: AtCmdStr,
    // Buffer for at command responses
    resp_buffer: AtCmdStr,
    // Buffer to store command arguments
    argument_buffer: AtCmdStr,
}

impl AtCmdResp {
    // Constructor which adds all supported AT commands
    pub fn new() -> AtCmdResp {
        AtCmdResp {
            cmd_buffer: AtCmdStr::new(),
            resp_buffer: AtCmdStr::new(),
            argument_buffer: AtCmdStr::new(),
        }        
    } 

    // Function to process 1 char at a time. Add them to internal buffers and decode AT commands.
    //
    // Returns tuple of: (decoded at command enum, t/f print help)
    pub fn handle_rx_char(&mut self, in_char: u8) -> Option<(AtCommand,bool)> {
        let mut command_accepted = AtCommand::Unknown;
        let mut print_help = false;
        
        // If enter character is received, handle command
        if in_char == CARRIAGE_RETURN {
            if self.cmd_buffer.len() == 0 {
                //defmt::info!("new line");
                return Some((AtCommand::NewLine, false));
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

                // Clear the buffer and load >
                self.resp_buffer.clear();
                unwrap!(self.resp_buffer.push_str("\n\r>"));

                // Clear command buffer
                self.cmd_buffer.clear();

                return None;
            }
            
            // Handle Help Commands
            if command_str.contains("?") {
                // AT? is special case
                if command_str == "AT?" {
                    command_accepted = AtCommand::AtList;
                }
                else {
                    let truncated_str = &command_str[0..command_str.len() - 1];
                    if let Some(found_cmd) = AtCommand::match_command(truncated_str) {
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
                if let Some(found_cmd) = AtCommand::match_command(split_str[0]) {
                    if AtCommand::allow_write(found_cmd) {
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
            else if let Some(found_cmd) = AtCommand::match_command(command_str) {
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

    pub fn prepare_response(&mut self, resp_enum: AtCommand, resp_val: &str) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.push_str("\n\r"));

        // Add pre canned response str
        if let Some(resp) = AtCommand::get_response(resp_enum) {
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
        unwrap!(self.resp_buffer.push_str("\n\rOK"));
        unwrap!(self.resp_buffer.push_str("\n\r>"));

        self.resp_buffer.as_bytes()
    }

    pub fn prepare_help_str(&mut self, resp_enum: AtCommand) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.push_str("\n\r"));

        // Add pre canned help str
        if let Some(resp) = AtCommand::get_help(resp_enum) {
            if resp.len() > 2 {
                unwrap!(self.resp_buffer.push_str(resp));
            }
        }

        unwrap!(self.resp_buffer.push_str("\n\r>"));
        
        self.resp_buffer.as_bytes()
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
