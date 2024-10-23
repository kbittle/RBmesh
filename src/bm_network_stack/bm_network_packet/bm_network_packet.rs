use heapless::Vec;
use bitfield_struct::bitfield;
use crate::bm_network_stack::NetworkId;

// Sizeof(BmNetworkPacketHdr)
const BM_PACKET_HDR_SIZE: u8 = 19;

// Max number of bytes paylaod can support. This should be 255 - sizeof(hdr).
const BM_MAX_PAYLOAD_SIZE: usize = 200;

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
impl BmPacketTypes {
    // This has to be a const fn
    const fn into_bits(self) -> u8 {
        self as _
    }
    const fn from_bits(value: u8) -> Self {
        match value {
            10 => Self::RouteDiscoveryRequest,
            11 => Self::RouteDiscoveryResponse,
            12 => Self::RouteDiscoveryError,
            _ => Self::BcastNeighborTable,
        }
    }
}

#[bitfield(u8)]
#[derive(PartialEq, Eq)]
pub struct BmNetworkHdrInfo {
    // "time to live", i.e. the number of hops allowed before message is discarded by a node
    #[bits(3)]
    ttl: u8,
    // Number of hops that the sender is from the source, this is incremented by the receiver
    #[bits(3)]
    hop_count: u8,
    // Flag requesting ack to packet. Note only used by some packet types
    #[bits(default = false)]
    required_ack: bool,
    // padding bit
    #[bits(1)]
    __: u8
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct BmNetworkPacketHdr {
    pub src: NetworkId,
    pub next_hop: NetworkId,
    pub orig: NetworkId,
    pub dest: NetworkId,
    pub info: BmNetworkHdrInfo,
    pub packet_type: BmPacketTypes,
    length: u8,
}

impl BmNetworkPacketHdr {
    pub fn new() -> BmNetworkPacketHdr {
        BmNetworkPacketHdr { 
            src: None,
            next_hop: None,
            orig: None,
            dest: None,
            info: BmNetworkHdrInfo::new()
                .with_ttl(7)
                .with_hop_count(0)
                .with_required_ack(false),
            packet_type: BmPacketTypes::default(),
            length: 0,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct BmNetworkPacket {
    pub hdr: BmNetworkPacketHdr,
    payload: Vec<u8, BM_MAX_PAYLOAD_SIZE>,
}

impl BmNetworkPacket {
    pub fn new() -> BmNetworkPacket {
        BmNetworkPacket {
            hdr: BmNetworkPacketHdr::new(),
            payload: Vec::new(),
        }
    }

    pub fn from(length: u8, buffer: &mut [u8]) -> Option<BmNetworkPacket> {
        // Ensure packet is long enough to contain the header
        if length < BM_PACKET_HDR_SIZE {
            return None
        }

        // Create vec from paylaod bytes
        let mut payload = Vec::new();
        // Todo - figure out better way to get u8 to vec with const
        payload.extend_from_slice(buffer[18..].try_into().unwrap()).unwrap();

        // Todo parse packet into pieces below
        Some(BmNetworkPacket {
                hdr: BmNetworkPacketHdr {
                    dest: Some(u32::from_be_bytes(buffer[0..3].try_into().unwrap())),
                    src: Some(u32::from_be_bytes(buffer[4..7].try_into().unwrap())),
                    next_hop: Some(u32::from_be_bytes(buffer[8..11].try_into().unwrap())),
                    orig: Some(u32::from_be_bytes(buffer[12..15].try_into().unwrap())),
                    info: BmNetworkHdrInfo(buffer[16]),
                    packet_type: BmPacketTypes::from_bits(buffer[17]),
                    length: buffer[18],
                },
                payload,
            }
        )
    }

    pub fn to_bytes(&mut self) -> Option<Vec<u8, 220>> {
        let mut out_buffer: Vec<u8, 220> = Vec::new();

        // Copy packet to vector buffer
        if out_buffer.extend_from_slice(&self.hdr.dest.unwrap().to_le_bytes()).is_err() { return None; }
        if out_buffer.extend_from_slice(&self.hdr.src.unwrap().to_le_bytes()).is_err() { return None; }
        if out_buffer.extend_from_slice(&self.hdr.next_hop.unwrap().to_le_bytes()).is_err() { return None; }
        if out_buffer.extend_from_slice(&self.hdr.orig.unwrap().to_le_bytes()).is_err() { return None; }
        if out_buffer.push(self.hdr.info.into()).is_err() { return None; }
        if out_buffer.push(self.hdr.packet_type.clone() as u8).is_err() { return None; }
        if out_buffer.push(self.hdr.length).is_err() { return None; }
        
        for &payload_byte in self.payload.iter() {
            out_buffer.push(payload_byte).unwrap();
        }

        // Return the length of bytes to send
        Some(out_buffer)
    }
}
