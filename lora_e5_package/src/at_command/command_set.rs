use heapless::{self, String, Vec}; // fixed capacity `std::Vec`
use defmt::{write, unwrap};

// Max AT command/response size calculation:
// - AT+MSEND input: "AT+MSEND=" (9) + dest_id (10) + ",true,7," (8) + payload (200) = 227
// - AT+MRECV response: "\n\r+" (3) + originator (10) + ",7,-128,200," (16) + payload (200) + "\n\rOK\n\r>" (8) = 237
// - Routing table/other responses well under 100 chars
// 250 provides ~13 chars headroom
const MAX_AT_CMD_CHARS: usize = 250;

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
    &["AT+FREQ", "+FREQ:", "Command to get/set frequency (Hz).\n\rFormat: AT+FREQ=<frequency>", "Y"],    
    &["AT+PWR", "+PWR:", "Command to get/set TX power (dBm).\n\rFormat: AT+PWR=<power>", "Y"],
    &["AT+VER", "VER:", "Command to get firmware version.", "N"],
    &["AT+BW", "+BW:", "Command to get/set LoRa bandwidth (kHz).\n\rFormat: AT+BW=<7|10|15|20|31|41|62|125|250|500>", "Y"],
    &["AT+CR", "+CR:", "Command to get/set LoRa coding rate (4/x).\n\rFormat: AT+CR=<4|5|6|7|8>", "Y"],
    &["AT+RING", "+RING: ", "Command to enable/disable ring indicator.", "Y"],
    &["AT+ROUTES", "+ROUTES:", "Command to show detailed route info per node (RSSI, distance, routes).", "N"],
    &["AT+NODES", "+NODES:", "Command to print out node list.", "N"],
    &["AT+ST", "+ST:", "Command to get radio status.", "N"],
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
    AtFreq,
    AtPwr,
    AtVer,
    AtBw,
    AtCr,
    RingIndicator,
    AtRoutes,
    AtNodes,
    AtSt,
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
            AtCommandSet::AtFreq => write!(fmt, "AtFreq"),
            AtCommandSet::AtPwr => write!(fmt, "AtPwr"),
            AtCommandSet::AtVer => write!(fmt, "AtVer"),
            AtCommandSet::AtBw => write!(fmt, "AtBw"),
            AtCommandSet::AtCr => write!(fmt, "AtCr"),
            AtCommandSet::RingIndicator => write!(fmt, "RingIndicator"),
            AtCommandSet::AtRoutes => write!(fmt, "AtRoutes"),
            AtCommandSet::AtNodes => write!(fmt, "AtNodes"),
            AtCommandSet::AtSt => write!(fmt, "AtSt"),
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
            8 => AtCommandSet::AtFreq,
            9 => AtCommandSet::AtPwr,
            10 => AtCommandSet::AtVer,
            11 => AtCommandSet::AtBw,
            12 => AtCommandSet::AtCr,
            13 => AtCommandSet::RingIndicator,
            14 => AtCommandSet::AtRoutes,
            15 => AtCommandSet::AtNodes,
            16 => AtCommandSet::AtSt,
            17 => AtCommandSet::AtList,
            18 => AtCommandSet::NewLine,
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

// Compile-time assertion: verify enum count matches array count
const _: () = assert!(CONST_AT_COMMAND_STRINGS.len() == 18, 
    "CONST_AT_COMMAND_STRINGS length must match AtCommandSet enum variants (excluding NewLine/Unknown)");
