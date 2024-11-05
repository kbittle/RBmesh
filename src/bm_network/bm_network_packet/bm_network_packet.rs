use heapless::Vec;
use bitfield_struct::bitfield;
use crate::bm_network::{
    NetworkId, 
    bm_network_configs::*,
};
use core::fmt::{self};

// Buffer size of hdr + payload
pub type BmNetworkOtaPacket = Vec<u8, BM_MAX_OTA_SIZE>;
// Vec type with just the size3 of the payload
pub type BmNetworkPacketPayload = Vec<u8, BM_MAX_PAYLOAD_SIZE>;

// Max TTL and hop count value
const MAX_TTL_HOP_CNT: u8 = 7;

#[repr(u8)]
#[derive(Default, Clone, Debug, PartialEq)]
pub enum BmPacketTypes {
    #[default]
    BcastNeighborTable = 0,

    RouteDiscoveryRequest = 10,
    RouteDiscoveryResponse = 11,
    RouteDiscoveryError = 12,

    DataPayload = 20,
    DataPayloadAck = 21,
}

impl fmt::Display for BmPacketTypes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BmPacketTypes::BcastNeighborTable => {
                write!(f, "BcastNeighborTable")
            }
            BmPacketTypes::RouteDiscoveryRequest => {
                write!(f, "RouteDiscoveryRequest")
            }
            BmPacketTypes::RouteDiscoveryResponse => {
                write!(f, "RouteDiscoveryResponse")
            }
            BmPacketTypes::DataPayload => {
                write!(f, "DataPayload")
            }
            BmPacketTypes::DataPayloadAck => {
                write!(f, "DataPayloadAck")
            }
            _ => { write!(f, "Unknown") }
        }
    }
}

impl BmPacketTypes {
    const fn from_bits(value: u8) -> Self {
        match value {
            0 => Self::BcastNeighborTable,
            
            10 => Self::RouteDiscoveryRequest,
            11 => Self::RouteDiscoveryResponse,
            12 => Self::RouteDiscoveryError,

            20 => Self::DataPayload,
            21 => Self::DataPayloadAck,

            _ => Self::BcastNeighborTable,
        }
    }
}

#[bitfield(u8)]
#[derive(PartialEq, Eq)]
pub struct BmNetworkHdrInfo {
    // "time to live", i.e. the number of hops allowed before message is discarded by a node
    #[bits(3)]
    pub ttl: u8,
    // Number of hops that the sender is from the source, this is incremented by the receiver
    #[bits(3)]
    pub hop_count: u8,
    // Flag requesting ack to packet. Note only used by some packet types
    #[bits(default = false)]
    pub required_ack: bool,
    // Flag indicating payload is encrypted
    #[bits(1)]
    pub encrypted: bool
}

// Packet routing structure
#[derive(Default, Debug, Clone, PartialEq)]
pub struct BmNetworkRoutingHdr {
    src: NetworkId,
    next_hop: NetworkId,
    orig: NetworkId,
    dest: NetworkId,
    info: BmNetworkHdrInfo,
}

impl BmNetworkRoutingHdr {
    // Constructor
    pub fn new(ttl: u8, ack: bool) -> BmNetworkRoutingHdr {
        BmNetworkRoutingHdr { 
            src: None,
            next_hop: None,
            orig: None,
            dest: None,
            info: BmNetworkHdrInfo::new()
                .with_ttl(ttl)
                .with_hop_count(0)
                .with_required_ack(ack)
                .with_encrypted(false),
        }
    }

    pub const fn with_src(mut self, new_src: NetworkId) -> Self {
        self.src = new_src;
        self
    }

    pub const fn with_next_hop(mut self, new_next_hop: NetworkId) -> Self {
        self.next_hop = new_next_hop;
        self
    }

    pub const fn with_orig(mut self, new_orig: NetworkId) -> Self {
        self.orig = new_orig;
        self
    }

    pub const fn with_dest(mut self, new_dest: NetworkId) -> Self {
        self.dest = new_dest;
        self
    }

    pub fn set_ttl(&mut self, new_ttl: u8) {
        self.info.set_ttl(new_ttl);
    }

    pub fn set_ack_required(&mut self, new_ack_required: bool) {
        self.info.set_required_ack(new_ack_required);
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
pub enum TransmitState {
    #[default]
    Waiting,
    Ok,
    Complete,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct BmNetworkPacket {
    // Packet enumeration
    pub packet_type: BmPacketTypes,
    // Routing header
    routing_hdr: BmNetworkRoutingHdr,
    // Payload buffer, optional as only data payload will use this
    payload: Option<BmNetworkPacketPayload>,

    // Metadata (Note: Does not go OTA)
    pub tx_state: TransmitState,
    pub tx_complete_timestamp: Option<i64>,
    pub tx_count: u8,
    pub wait_for_reply: bool,
}

impl fmt::Display for BmNetworkPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Type:{}, Src:{}, Dst:{}, Ack:{}", 
            self.packet_type, 
            self.routing_hdr.src.unwrap_or(0), 
            self.routing_hdr.dest.unwrap_or(0), 
            self.routing_hdr.info.required_ack()
        )
    }
}

impl BmNetworkPacket {
    // Constructor
    pub fn new(new_packet_type: BmPacketTypes, orig: NetworkId, next_hop: NetworkId, dest: NetworkId, ttl: u8, ack: bool, new_payload: Option<BmNetworkPacketPayload>) -> Self {
        BmNetworkPacket {
            packet_type: new_packet_type,
            routing_hdr: BmNetworkRoutingHdr::new(ttl, ack)
                .with_src(orig)
                .with_next_hop(next_hop)
                .with_orig(orig)
                .with_dest(dest),
            payload: new_payload,
            tx_state: TransmitState::Waiting,
            tx_complete_timestamp: None,
            tx_count: 0,
            wait_for_reply: false,
        }
    }

    pub const fn with_next_hop(mut self, new_next_hop: NetworkId) -> Self {
        self.routing_hdr = self.routing_hdr.with_next_hop(new_next_hop);
        self
    }

    pub const fn with_ok_to_transmit(mut self) -> Self {
        self.tx_state = TransmitState::Ok;
        self
    }

    pub const fn with_wait_for_reply(mut self) -> Self {
        self.wait_for_reply = true;
        self
    }

    // Public accessor functions
    pub fn get_source(&mut self) -> NetworkId {
        self.routing_hdr.src
    }
    pub fn set_source(&mut self, new_src: NetworkId) {
        self.routing_hdr.src = new_src;
    }
    pub fn get_next_hop(&mut self) -> NetworkId {
        self.routing_hdr.next_hop
    }
    pub fn set_next_hop(&mut self, new_next_hop: NetworkId) {
        self.routing_hdr.next_hop = new_next_hop;
    }
    pub fn get_originator(&mut self) -> NetworkId {
        self.routing_hdr.orig
    }
    pub fn get_destination(&mut self) -> NetworkId {
        self.routing_hdr.dest
    }
    pub fn get_hop_count(&mut self) -> u8 {
        self.routing_hdr.info.hop_count()
    }
    pub fn get_info(&mut self) -> BmNetworkHdrInfo {
        self.routing_hdr.info
    }
    pub fn set_info(&mut self, new_info: BmNetworkHdrInfo) {
        self.routing_hdr.info = new_info;
    }
    pub fn increment_hop_count(&mut self) {
        let hop_cnt = self.routing_hdr.info.hop_count();

        if hop_cnt + 1 <= MAX_TTL_HOP_CNT {
            self.routing_hdr.info.set_hop_count(hop_cnt + 1);
        }
        
    }
    pub fn set_ok_to_transmit(&mut self) {
        self.tx_state = TransmitState::Ok;
    }
    pub fn is_ok_to_transmit(&mut self) -> bool {
        self.tx_state == TransmitState::Ok
    }
    pub fn set_wait_for_reply(&mut self) {
        self.wait_for_reply = true;
    }
    pub fn is_waiting_for_reply(&mut self) -> bool {
        self.wait_for_reply
    }

    // Mutation functions
    pub fn from(length: usize, buffer: &mut [u8]) -> Option<BmNetworkPacket> {
        // Ensure packet is long enough to contain the header
        if length < BM_PACKET_HDR_SIZE {
            defmt::warn!("BmNetworkPacket: len too small");
            return None
        }

        defmt::info!("from: buffer={}", buffer[0..length]);

        // Create vec from payload bytes
        let mut payload_vec: BmNetworkPacketPayload = Vec::new();
        let mut payload: Option<BmNetworkPacketPayload> = None;
        if length > BM_PACKET_HDR_SIZE {
            // Todo - figure out better way to get u8 to vec with const
            payload_vec.extend_from_slice(buffer[BM_PACKET_HDR_SIZE..length].try_into().unwrap()).unwrap();
            payload = Some(payload_vec);
        }       

        // Todo parse packet into pieces below
        Some(BmNetworkPacket {
            packet_type: BmPacketTypes::from_bits(buffer[0]),
                routing_hdr: BmNetworkRoutingHdr {
                    dest: Some(u32::from_ne_bytes(buffer[1..5].try_into().unwrap())),
                    src: Some(u32::from_ne_bytes(buffer[5..9].try_into().unwrap())),
                    next_hop: Some(u32::from_ne_bytes(buffer[9..13].try_into().unwrap())),
                    orig: Some(u32::from_ne_bytes(buffer[13..17].try_into().unwrap())),
                    info: BmNetworkHdrInfo(buffer[17]),
                },
                payload,
                // Init metadata
                tx_state: TransmitState::Waiting,
                tx_complete_timestamp: None,
                tx_count: 0,
                wait_for_reply: false,
            }
        )
    }

    pub fn to_bytes(&mut self) -> Option<BmNetworkOtaPacket> {
        let mut out_buffer: BmNetworkOtaPacket = Vec::new();

        // Copy packet to vector buffer
        if out_buffer.push(self.packet_type.clone() as u8).is_err() { return None; }
        if out_buffer.extend_from_slice(&self.routing_hdr.dest.unwrap_or(0).to_ne_bytes()).is_err() { return None; }
        if out_buffer.extend_from_slice(&self.routing_hdr.src.unwrap_or(0).to_ne_bytes()).is_err() { return None; }
        if out_buffer.extend_from_slice(&self.routing_hdr.next_hop.unwrap_or(0).to_ne_bytes()).is_err() { return None; }
        if out_buffer.extend_from_slice(&self.routing_hdr.orig.unwrap_or(0).to_ne_bytes()).is_err() { return None; }
        if out_buffer.push(self.routing_hdr.info.into()).is_err() { return None; }

        // If there is a payload, oush bytes
        if let Some(payload) = self.payload.as_ref() {        
            for &payload_byte in payload.iter() {
                out_buffer.push(payload_byte).unwrap();
            }
        }

        defmt::info!("to: buffer={}", out_buffer[0..out_buffer.len()]);     

        // Return the length of bytes to send
        Some(out_buffer)
    }
}
