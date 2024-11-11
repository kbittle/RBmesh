use heapless::{self, String, Vec}; // fixed capacity `std::Vec`
use defmt::{write, unwrap};

// TODO - figure out max resp size. Currently recv msg can be payload of 200 + w/e
// Configurable hard coded max at command length
const MAX_AT_CMD_CHARS: usize = 300;

// AT Command buffer type
pub type AtCmdStr = String<MAX_AT_CMD_CHARS>;

// Constant AT Command strings
const CONST_AT_COMMAND_STRINGS: &'static [&'static [&str]] = 
&[
    // Cmd, Resp, Help, Allow Write
    &["AT", "", "", "N"],
    &["AT+CSQ", "+CSQ:", "Command to get instantaneous RSSI.", "N"],
    &["AT+GMR", "Version:", "", "N"],
    &["AT+ID", "+ID:", "Enter Network ID as a 32bit value.", "N"],
    &["AT+MCNT", "+COUNT: ", "Command to receive message from network.", "N"],
    &["AT+MRECV", "+RX: ", "Command to receive message from network.", "N"],
    &["AT+MSEND", "", "Command to send message over mesh network.\n\rFormat: <dest id>,<ack required>,<ttl>,<payload>", "Y"],
    &["AT+TMSG", "+", "Command to send \"Hello World\".", "N"],
    &["AT+RCFG", "+CFG", "Command to get/set radio config.\n\rFormat:AT+RCFG=<sub cmd>,<sub value>\n\rSub Commands: FREQ|SF|CR|BW|PWR", "N"],    
    &["AT+RING", "+RING: ", "Command to enable/disable ring indicator.", "Y"],
    &["AT+RTABLE", "", "Command to print out routing table.", "N"],
    &["AT+ST", "+", "Command to get radio status.", "N"],
    &["AT?", "", "Command to get list of available commands.", "N"],
];

/// Enumeration of available AT commands.
// Note: When adding a new command:
//       - add new enum
//       - update CONST_AT_COMMAND_STRINGS
//       - update defmt function
//       - update AtCommand::from()
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtCommandSet {
    At,
    AtCsq,
    AtGmr,
    AtId,
    AtMsgReceiveCnt,
    AtMsgReceive,
    AtMsgSend,
    TestMessage,
    RadioConfiguration,
    RingIndicator,
    RoutingTable,
    RadioStatus,
    AtList,

    // Below are not in CONST_AT_COMMAND_STRINGS
    NewLine,
    Unknown,
}

impl defmt::Format for AtCommandSet {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            AtCommandSet::At => write!(fmt, "At"),
            AtCommandSet::AtCsq => write!(fmt, "AtCsq"),
            AtCommandSet::AtGmr => write!(fmt, "AtGmr"),
            AtCommandSet::AtId => write!(fmt, "AtId"),
            AtCommandSet::AtMsgReceiveCnt => write!(fmt, "AtMsgReceiveCnt"),
            AtCommandSet::AtMsgReceive => write!(fmt, "AtMsgReceive"),
            AtCommandSet::AtMsgSend => write!(fmt, "AtMsgSend"),
            AtCommandSet::TestMessage => write!(fmt, "TestMessage"),
            AtCommandSet::RadioConfiguration => write!(fmt, "RadioConfiguration"),
            AtCommandSet::RingIndicator => write!(fmt, "RingIndicator"),
            AtCommandSet::RoutingTable => write!(fmt, "RoutingTable"),
            AtCommandSet::RadioStatus => write!(fmt, "RadioStatus"),

            AtCommandSet::AtList => write!(fmt, "AtList"),
            AtCommandSet::NewLine => write!(fmt, "NewLine"),
            _ => { write!(fmt, "Unknown") }
        }
    }
}

impl AtCommandSet {
    const fn to_u8(self) -> usize {
        self as _
    }
    const fn from(index: usize) -> Self {
        match index {
            0 => AtCommandSet::At,
            1 => AtCommandSet::AtCsq,
            2 => AtCommandSet::AtGmr,
            3 => AtCommandSet::AtId,
            4 => AtCommandSet::AtMsgReceiveCnt,
            5 => AtCommandSet::AtMsgReceive,
            6 => AtCommandSet::AtMsgSend,
            7 => AtCommandSet::TestMessage,
            8 => AtCommandSet::RadioConfiguration,
            9 => AtCommandSet::RingIndicator,
            10 => AtCommandSet::RoutingTable,
            11 => AtCommandSet::RadioStatus,
            12 => AtCommandSet::AtList,
            13 => AtCommandSet::NewLine,
            _ => AtCommandSet::Unknown,
        }
    }

    pub fn match_command(command: &str) -> Option<Self> {
        for (index, entry) in CONST_AT_COMMAND_STRINGS.iter().enumerate() {
            if entry[0] == command {
                return Some(AtCommandSet::from(index));
            }
        }
        None
    }

    pub fn get_command(cmd: AtCommandSet) -> Option<&'static str> {    
        if cmd.to_u8() < CONST_AT_COMMAND_STRINGS.len() {
            Some(CONST_AT_COMMAND_STRINGS[cmd.to_u8()][0])
        } else {
            None
        }
    }
    
    pub fn get_response(cmd: AtCommandSet) -> Option<&'static str> {    
        if cmd.to_u8() < CONST_AT_COMMAND_STRINGS.len() {
            Some(CONST_AT_COMMAND_STRINGS[cmd.to_u8()][1])
        } else {
            None
        }
    }
    
    pub fn get_help(cmd: AtCommandSet) -> Option<&'static str> {    
        if cmd.to_u8() < CONST_AT_COMMAND_STRINGS.len() {
            Some(CONST_AT_COMMAND_STRINGS[cmd.to_u8()][2])
        } else {
            None
        }
    }

    pub fn allow_write(cmd: AtCommandSet) -> bool {
        if cmd.to_u8() < CONST_AT_COMMAND_STRINGS.len() {
            CONST_AT_COMMAND_STRINGS[cmd.to_u8()][3] == "Y"
        } else {
            false
        }
    }

    // Appends a ref string with all available commands in the local list
    pub fn get_available_cmds(str_out: &mut AtCmdStr) {
        unwrap!(str_out.push_str("Available Commands:"));

        // Append all supported commands
        for entry in CONST_AT_COMMAND_STRINGS.iter() {
            unwrap!(str_out.push_str("\n\r"));
            unwrap!(str_out.push_str(entry[0]));
        }
    }
}
