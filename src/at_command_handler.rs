use heapless::{self, Vec, String}; // fixed capacity `std::Vec`
use defmt::unwrap; // global logger

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtCommandSet {
    CmdUnknown,
    CmdNewLine,
    CmdAt,
    CmdAtCsq,
    CmdAtGmr,
    CmdAtId,    
    CmdSendMessage,
    CmdRadioStatus,
    CmdList
}

// Configurable hard coded max at command length
const MAX_AT_CMD_CHARS: usize = 100;

// Configurable hard coded max at command length
const MAX_AT_CMDS: usize = 25;

type AtCmdBuffer = Vec<u8, 512>;
pub type AtCmdStr = String<MAX_AT_CMD_CHARS>;
pub type AtCmdList = Vec<AtCmd, MAX_AT_CMDS>;

#[derive(Debug, Clone, PartialEq)]
pub struct AtCmd {
    command_enum: AtCommandSet,
    command_str: AtCmdStr,
    allows_write: bool,
    response_str: AtCmdStr
}

// Default constructor
impl Default for AtCmd {
    fn default() -> Self {
        AtCmd {
            command_enum: AtCommandSet::CmdUnknown,
            command_str: AtCmdStr::default(),
            allows_write: false,
            response_str: AtCmdStr::default(),
        }
    }
}

impl AtCmd {
    pub fn new(
        command_enum: AtCommandSet,
        command_str: AtCmdStr,
        allows_write: bool,
        response_str: AtCmdStr,
    ) -> AtCmd {
        AtCmd {
            command_enum,
            command_str,
            allows_write,
            response_str,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct AtCmdResp {
    // Buffer for incoming at commands
    cmd_buffer: AtCmdBuffer,
    // Buffer for at command responses
    resp_buffer: AtCmdBuffer,
    // List of registered at commands
    supported_at_cmd: AtCmdList,
}

impl AtCmdResp {
    pub fn new() -> AtCmdResp {
        AtCmdResp {
            cmd_buffer: AtCmdBuffer::new(),
            resp_buffer: AtCmdBuffer::new(),
            supported_at_cmd: AtCmdList::new(),
        }
    }    

    pub fn add_at_cmd(&mut self, command_enum: AtCommandSet, command_str: &str, allow_write: bool, response_str: &str) {
        if self.supported_at_cmd.len() < MAX_AT_CMDS {
            self.supported_at_cmd.push(
                AtCmd::new(
                    command_enum,
                    String::try_from(command_str).unwrap(),
                    allow_write,
                    String::try_from(response_str).unwrap()
                )
            ).unwrap();
        } else {
            defmt::error!("Error: Maximum number of AT commands reached.");
        }
    }   

    pub fn handle_command(&mut self, in_char: u8) -> AtCommandSet {
        let mut command_accepted = AtCommandSet::CmdUnknown;
        
        // If enter character is received, handle command
        if in_char == b'\r' {
            if self.cmd_buffer.len() == 0 {
                defmt::info!("new line");
                return AtCommandSet::CmdNewLine;
            }
            else if self.cmd_buffer.len() < 2 {
                defmt::info!("Command too short");

                // Clear command buffer
                self.cmd_buffer.clear();

                return command_accepted;
            }

            // Turn into slice
            let command_to_process = self.cmd_buffer.clone();
            let command_str = core::str::from_utf8(&command_to_process).unwrap();
            defmt::info!("Command received: {}", command_str);

            // Check for AT at start
            let at_str: &str = &command_str[..2];
            if at_str != "AT" {
                defmt::info!("Missing AT at start of string.");

                // Clear the buffer and load >
                self.resp_buffer.clear();
                unwrap!(self.resp_buffer.extend_from_slice("\n\r>".as_bytes()));

                // Clear command buffer
                self.cmd_buffer.clear();

                return command_accepted;
            }
            
            // Handle command
            let cmd_str: &str = &command_str[2..];
            for supp_cmd in &self.supported_at_cmd.clone() {
                if supp_cmd.allows_write {
                    // Create a copy of str to memcmp
                    let mut str_to_match = supp_cmd.command_str.clone();
                    match str_to_match.push_str("=").map_err(|_| "Failed to append the string.") {
                        Ok(_) => {
                            if cmd_str.contains(supp_cmd.command_str.trim()) {
                                // Todo - parse value after =
                            }
                        },
                        Err(e) => { 
                            defmt::error!("String.push_str failed!");
                        }
                    }                    
                }

                // Todo add check for comd + ?. I.E. AT+CSQ or AT+CSQ?

                // Return matched command
                if cmd_str == supp_cmd.command_str.trim() {
                    command_accepted = supp_cmd.command_enum;
                }
            }

            // Clear command buffer
            self.cmd_buffer.clear();
        }
        else {            
            // Add char to in buffer
            unwrap!(self.cmd_buffer.push(in_char));
        }

        command_accepted
    }

    pub fn prepare_response(&mut self, resp_enum: AtCommandSet, resp_val: &str) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.extend_from_slice("\n\r".as_bytes()));

        // Add pre canned response str
        for supp_cmd in &self.supported_at_cmd.clone() {
            if supp_cmd.command_enum == resp_enum {
                unwrap!(self.resp_buffer.extend_from_slice(supp_cmd.response_str.as_bytes()));
            }
        }
        
        // Add response value to buffer
        unwrap!(self.resp_buffer.extend_from_slice(resp_val.as_bytes()));

        // Add generic OK and >
        unwrap!(self.resp_buffer.extend_from_slice("\n\rOK".as_bytes()));
        unwrap!(self.resp_buffer.extend_from_slice("\n\r>".as_bytes()));

        self.resp_buffer.as_slice()
    }

    pub fn get_available_cmds(&mut self) -> AtCmdStr {
        let mut str_out = AtCmdStr::new();

        unwrap!(str_out.push_str("Available Commands:"));

        // Append all supported commands
        for supp_cmd in &self.supported_at_cmd.clone() {
            unwrap!(str_out.push_str("\n\rAT"));
            unwrap!(str_out.push_str(supp_cmd.command_str.as_str()));
        }

        str_out.clone()
    }

    //-----------------------------------------------------------
    // Private functions
    //-----------------------------------------------------------   

}
