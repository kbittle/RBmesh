use heapless::{self, String, Vec}; // fixed capacity `std::Vec`
use defmt::{write, unwrap};
use bm_network::{
    NetworkId, 
    bm_network_packet::bm_network_packet::BmNetworkPacketPayload
};

// Configurable hard coded max at command length
const MAX_AT_CMD_CHARS: usize = 200;

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

// Supported AT commands.

// Note: When adding a new command:
//       - add new enum
//       - update CONST_AT_COMMAND_STRINGS
//       - update defmt function
//       - update AtCommand::from()
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtCommand {
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

impl defmt::Format for AtCommand {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            AtCommand::At => write!(fmt, "At"),
            AtCommand::AtCsq => write!(fmt, "AtCsq"),
            AtCommand::AtGmr => write!(fmt, "AtGmr"),
            AtCommand::AtId => write!(fmt, "AtId"),
            AtCommand::AtMsgReceiveCnt => write!(fmt, "AtMsgReceiveCnt"),
            AtCommand::AtMsgReceive => write!(fmt, "AtMsgReceive"),
            AtCommand::AtMsgSend => write!(fmt, "AtMsgSend"),
            AtCommand::TestMessage => write!(fmt, "TestMessage"),
            AtCommand::RadioConfiguration => write!(fmt, "RadioConfiguration"),
            AtCommand::RingIndicator => write!(fmt, "RingIndicator"),
            AtCommand::RoutingTable => write!(fmt, "RoutingTable"),
            AtCommand::RadioStatus => write!(fmt, "RadioStatus"),

            AtCommand::AtList => write!(fmt, "AtList"),
            AtCommand::NewLine => write!(fmt, "NewLine"),
            _ => { write!(fmt, "Unknown") }
        }
    }
}

impl AtCommand {
    const fn to_u8(self) -> usize {
        self as _
    }
    const fn from(index: usize) -> Self {
        match index {
            0 => AtCommand::At,
            1 => AtCommand::AtCsq,
            2 => AtCommand::AtGmr,
            3 => AtCommand::AtId,
            4 => AtCommand::AtMsgReceiveCnt,
            5 => AtCommand::AtMsgReceive,
            6 => AtCommand::AtMsgSend,
            7 => AtCommand::TestMessage,
            8 => AtCommand::RadioConfiguration,
            9 => AtCommand::RingIndicator,
            10 => AtCommand::RoutingTable,
            11 => AtCommand::RadioStatus,
            12 => AtCommand::AtList,
            13 => AtCommand::NewLine,
            _ => AtCommand::Unknown,
        }
    }

    pub fn match_command(command: &str) -> Option<Self> {
        for (index, entry) in CONST_AT_COMMAND_STRINGS.iter().enumerate() {
            if entry[0] == command {
                return Some(AtCommand::from(index));
            }
        }
        None
    }

    pub fn get_command(cmd: AtCommand) -> Option<&'static str> {    
        if cmd.to_u8() < CONST_AT_COMMAND_STRINGS.len() {
            Some(CONST_AT_COMMAND_STRINGS[cmd.to_u8()][0])
        } else {
            None
        }
    }
    
    pub fn get_response(cmd: AtCommand) -> Option<&'static str> {    
        if cmd.to_u8() < CONST_AT_COMMAND_STRINGS.len() {
            Some(CONST_AT_COMMAND_STRINGS[cmd.to_u8()][1])
        } else {
            None
        }
    }
    
    pub fn get_help(cmd: AtCommand) -> Option<&'static str> {    
        if cmd.to_u8() < CONST_AT_COMMAND_STRINGS.len() {
            Some(CONST_AT_COMMAND_STRINGS[cmd.to_u8()][2])
        } else {
            None
        }
    }

    pub fn allow_write(cmd: AtCommand) -> bool {
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

pub type MessageTuple = (NetworkId, bool, u8, BmNetworkPacketPayload);

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
