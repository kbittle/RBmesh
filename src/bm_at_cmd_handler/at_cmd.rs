use heapless::{self, String, Vec}; // fixed capacity `std::Vec`
use defmt::{write, unwrap};
use super::super::bm_network::{
    NetworkId,
    bm_network_packet::bm_network_packet::BmNetworkPacketPayload
};

// Configurable hard coded max at command length
const MAX_AT_CMD_CHARS: usize = 200;

// AT Command buffer type
pub type AtCmdStr = String<MAX_AT_CMD_CHARS>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AtCommandSet {
    CmdUnknown,
    CmdNewLine,
    CmdAt,
    CmdAtCsq,
    CmdAtGmr,
    CmdAtId,
    CmdAtMsg,
    CmdTestMessage,
    CmdRoutingTable,
    CmdRadioStatus,
    CmdList,
}

impl defmt::Format for AtCommandSet {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            AtCommandSet::CmdNewLine => write!(fmt, "CmdNewLine"),
            AtCommandSet::CmdAt => write!(fmt, "CmdAt"),
            AtCommandSet::CmdAtCsq => write!(fmt, "CmdAtCsq"),
            AtCommandSet::CmdAtGmr => write!(fmt, "CmdAtGmr"),
            AtCommandSet::CmdAtId => write!(fmt, "CmdAtId"),
            AtCommandSet::CmdAtMsg => write!(fmt, "CmdAtMsg"),
            AtCommandSet::CmdTestMessage => write!(fmt, "CmdTestMessage"),
            AtCommandSet::CmdRoutingTable => write!(fmt, "CmdRoutingTable"),
            AtCommandSet::CmdRadioStatus => write!(fmt, "CmdRadioStatus"),
            _ => { write!(fmt, "CmdUnknown") }
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct AtCmd {
    pub command_enum: AtCommandSet,
    pub command_str: AtCmdStr,
    pub allows_write: bool,
    pub response_str: AtCmdStr,
    pub help_str: AtCmdStr,
}

// Default constructor
impl Default for AtCmd {
    fn default() -> Self {
        AtCmd {
            command_enum: AtCommandSet::CmdUnknown,
            command_str: AtCmdStr::default(),
            allows_write: false,
            response_str: AtCmdStr::default(),
            help_str: AtCmdStr::default(),
        }
    }
}

impl AtCmd {
    pub fn new(
        command_enum: AtCommandSet,
        command_str: AtCmdStr,
        allows_write: bool,
        response_str: AtCmdStr,
        help_str: AtCmdStr,
    ) -> AtCmd {
        AtCmd {
            command_enum,
            command_str,
            allows_write,
            response_str,
            help_str,
        }
    }
}

pub type MessageTuple = (NetworkId, bool, u8, BmNetworkPacketPayload);

pub fn cmd_arg_into_msg(argument_buffer: AtCmdStr) -> Option<MessageTuple> {
    // Expected format in the argument buffer: "dest,ack,ttl,ascii payload"
    let args: Vec<&str, 5> = argument_buffer.split(',').collect();
    
    defmt::error!("get_cmd_arg_as_msg: len={}", args.len());

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
        defmt::error!("Invalid number of arguments.");
    }
    None
}
