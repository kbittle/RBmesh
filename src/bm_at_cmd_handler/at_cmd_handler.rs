use heapless::{self, Vec, String}; // fixed capacity `std::Vec`
use defmt::unwrap;

use crate::bm_network::NetworkId; // global logger
use crate::bm_at_cmd_handler::at_cmd::{
    AtCmdStr,
    AtCmd,
    AtCommandSet,
};

// Configurable hard coded max at command length
const MAX_AT_CMDS: usize = 25;

pub type AtCmdList = Vec<AtCmd, MAX_AT_CMDS>;

#[derive(Clone, PartialEq)]
pub struct AtCmdResp {
    // Buffer for incoming at commands
    cmd_buffer: AtCmdStr,
    // Buffer for at command responses
    resp_buffer: AtCmdStr,
    // List of registered at commands
    supported_at_cmd: AtCmdList,
    // Buffer to store command arguments
    argument_buffer: AtCmdStr,
}

impl AtCmdResp {
    // Constructor which adds all supported AT commands
    pub fn new() -> AtCmdResp {
        AtCmdResp {
            cmd_buffer: AtCmdStr::new(),
            resp_buffer: AtCmdStr::new(),
            supported_at_cmd: AtCmdList::new(),
            argument_buffer: AtCmdStr::new(),
        }
        
    } 

    pub fn load_at_commands(&mut self) {
        self.add_at_cmd(AtCommandSet::CmdAt, "", false, "", "");
        self.add_at_cmd(AtCommandSet::CmdAtCsq, "+CSQ", false, "+CSQ:", "Command to get instantaneous RSSI.");
        self.add_at_cmd(AtCommandSet::CmdAtGmr, "+GMR", false, "Version:", "");
        self.add_at_cmd(AtCommandSet::CmdAtId, "+ID", false, "+ID:", "Enter Network ID as a 32bit value.");
        self.add_at_cmd(AtCommandSet::CmdAtMsg, "+MSG", true, "", "Format: <dest id>,<ack required>,<ttl>,<payload>");
        self.add_at_cmd(AtCommandSet::CmdTestMessage, "+TMSG", false, "+", "Command to send \"Hello World\".");
        self.add_at_cmd(AtCommandSet::CmdRoutingTable, "+RTABLE", false, "", "");
        self.add_at_cmd(AtCommandSet::CmdRadioStatus, "+ST", false, "+", "Command to get radio status.");
    }

    // Function to process 1 char at a time. Add them to internal buffers and decode AT commands.
    //
    // Returns tuple of: (decoded at command enum, t/f print help)
    pub fn handle_rx_char(&mut self, in_char: u8) -> Option<(AtCommandSet,bool)> {
        let mut command_accepted = AtCommandSet::CmdUnknown;
        let mut print_help = false;
        
        // If enter character is received, handle command
        if in_char == b'\r' {
            if self.cmd_buffer.len() == 0 {
                defmt::info!("new line");
                return Some((AtCommandSet::CmdNewLine, false));
            }
            else if self.cmd_buffer.len() < 2 {
                defmt::info!("Command too short");

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
            
            // Handle command
            let (at_str, cmd_str) = command_str.split_at(2);
            for supp_cmd in &self.supported_at_cmd {
                if supp_cmd.allows_write {
                    // Create a copy of str to memcmp
                    let mut str_to_match = supp_cmd.command_str.clone();
                    unwrap!(str_to_match.push('='));
                    // Check for cmd + '='
                    if cmd_str.contains(str_to_match.trim()) {
                        // Grab str after command
                        let arg: Vec<&str, 2> = cmd_str.split(str_to_match.trim()).collect();
                        if arg.len() > 1 {
                            defmt::info!("arguments: {}", arg[1]);
                            // Store arguments
                            self.argument_buffer.clear();
                            unwrap!(self.argument_buffer.push_str(arg[1]));
                            
                            command_accepted = supp_cmd.command_enum;
                        }
                    }                                                             
                }
                
                // Check for cmd + ?. I.E. AT+CSQ as AT+CSQ?
                let mut str_to_match = supp_cmd.command_str.clone();
                str_to_match.push('?').unwrap();
                if cmd_str == str_to_match {
                    defmt::info!("Command help: {}", cmd_str);
                    // Return Help as we dont want to print normal response
                    print_help = true;
                    command_accepted = supp_cmd.command_enum;
                }                
                else if cmd_str == supp_cmd.command_str.trim() {
                    defmt::info!("Command accepted: {}", cmd_str);
                    // Return matched command
                    command_accepted = supp_cmd.command_enum;
                }
            }

            // Clear command buffer
            self.cmd_buffer.clear();
            
            return Some((command_accepted, print_help))
        }
        else {            
            // Add char to in buffer
            self.cmd_buffer.push(char::try_from(in_char).unwrap()).unwrap();           
        }

        None
    }

    pub fn prepare_response(&mut self, resp_enum: AtCommandSet, resp_val: &str) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.push_str("\n\r"));

        // Add pre canned response str
        for supp_cmd in self.supported_at_cmd.iter_mut() {
            if supp_cmd.command_enum == resp_enum {
                // Stupid String library has a runtime error at pushing a str len of 1
                if supp_cmd.response_str.len() == 1 {
                    let ch = supp_cmd.response_str.chars().next().unwrap();
                    unwrap!(self.resp_buffer.push(ch));
                }
                if supp_cmd.response_str.len() > 2 {
                    unwrap!(self.resp_buffer.push_str(supp_cmd.response_str.as_str()));
                }                
            }
        }
        
        // Add response value to buffer
        unwrap!(self.resp_buffer.push_str(resp_val));

        // Add generic OK and >
        unwrap!(self.resp_buffer.push_str("\n\rOK"));
        unwrap!(self.resp_buffer.push_str("\n\r>"));

        self.resp_buffer.as_bytes()
    }

    pub fn prepare_help_str(&mut self, resp_enum: AtCommandSet) -> &[u8] {
        // Clear the buffer before loading new response
        self.resp_buffer.clear();

        // Convert the response string to bytes and extend the buffer
        unwrap!(self.resp_buffer.push_str("\n\r"));

        // Add pre canned help str
        for supp_cmd in self.supported_at_cmd.iter_mut() {
            if supp_cmd.command_enum == resp_enum {
                if supp_cmd.help_str.len() > 2 {
                    unwrap!(self.resp_buffer.push_str(supp_cmd.help_str.as_str()));
                }
            }
        }

        unwrap!(self.resp_buffer.push_str("\n\r>"));
        
        self.resp_buffer.as_bytes()
    }

    // Returns a string of all available commands in the local list
    pub fn get_available_cmds(&mut self) -> AtCmdStr {
        let mut str_out = AtCmdStr::new();

        unwrap!(str_out.push_str("Available Commands:"));

        // Append all supported commands
        for supp_cmd in self.supported_at_cmd.iter_mut() {
            unwrap!(str_out.push_str("\n\rAT"));
            unwrap!(str_out.push_str(supp_cmd.command_str.as_str()));
        }

        str_out.clone()
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

    fn add_at_cmd(&mut self, command_enum: AtCommandSet, command_str: &str, allow_write: bool, response_str: &str, help_str: &str) {
        // Create a new AtCmd instance
        let new_cmd = AtCmd::new(
            command_enum,
            String::try_from(command_str).unwrap(),  // Convert str to AtCmdStr
            allow_write,
            String::try_from(response_str).unwrap(),  // Convert str to AtCmdStr
            String::try_from(help_str).unwrap(),
        );

        // Add the new command to the supported commands list
        if self.supported_at_cmd.len() < MAX_AT_CMDS {
            self.supported_at_cmd.push(new_cmd).unwrap();
        } else {
            // Handle the case where the command list is full, if necessary
            defmt::error!("Warning: Maximum number of AT commands reached. Cannot add '{}'", command_str);
        }
    }  

}
