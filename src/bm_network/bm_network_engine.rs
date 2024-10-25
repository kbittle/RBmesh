use heapless::Vec; // fixed capacity `std::Vec`
use super::{
    bm_network_node::bm_network_node::BmNodeEntry, 
    bm_network_packet::bm_network_packet::{
        BmNetworkHdrInfo, 
        BmNetworkPacket, 
        BmNetworkRoutingHdr, 
        BmPacketTypes,
        BmNetworkPacketPayload
    }, 
    bm_network_stack::BmNetworkStack, 
    bm_network_configs::*,
    BmErrors, NetworkId, RssiType, TimeType
};
use defmt::{write, unwrap};

#[derive(Debug, Default, Clone, PartialEq)]
pub enum BmEngineStatus {
    #[default]
    Start,
    PerformingNetworkDiscovery,
    RouteFound,
    SendingPayload,
    RetryingPayload,
    WaitingForAck,
    AckReceieved,
    ErrorNoRoute,
    ErrorNoAck,
    Complete,
}

impl defmt::Format for BmEngineStatus {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            BmEngineStatus::Start => write!(fmt, "Start"),
            BmEngineStatus::PerformingNetworkDiscovery => write!(fmt, "PerformingNetworkDiscovery"),
            BmEngineStatus::RouteFound => write!(fmt, "RouteFound"),
            BmEngineStatus::SendingPayload => write!(fmt, "SendingPayload"),
            BmEngineStatus::RetryingPayload => write!(fmt, "RetryingPayload"),
            BmEngineStatus::WaitingForAck => write!(fmt, "WaitingForAck"),
            BmEngineStatus::AckReceieved => write!(fmt, "AckReceieved"),
            BmEngineStatus::ErrorNoRoute => write!(fmt, "ErrorNoRoute"),
            BmEngineStatus::ErrorNoAck => write!(fmt, "ErrorNoAck"),
            BmEngineStatus::Complete => write!(fmt, "Complete"),
            _ => { write!(fmt, "Unknown") }
        }
    }
}

pub struct BmNetworkEngine {
    pub stack: BmNetworkStack,

    // Out packet buffer
    outbound: Vec<BmNetworkPacket, BM_OUTBOUND_QUEUE_SIZE>,

    // Enum state machine for status of mesh engine
    engine_status: BmEngineStatus,
}

impl BmNetworkEngine {
    pub fn new(local_network_id: NetworkId) -> Self {
        BmNetworkEngine {
            stack: BmNetworkStack::new(local_network_id),
            outbound: Vec::new(),
            engine_status: BmEngineStatus::default(),
        }
    }

    pub fn process_packet(&mut self, length: usize, buffer: &mut [u8], millis: TimeType, rssi: RssiType) -> Option<BmNetworkPacket> {
        // Parse packet into struct
        // If we cannot successfully parse packet, return
        let mut new_packet = BmNetworkPacket::from(length, buffer)?;

        // Do not process our own packets
        if new_packet.get_originator() == self.stack.get_local_network_id() {
            return None
        }

        // Handle packet based off type
        match new_packet.packet_type {
            BmPacketTypes::RouteDiscoveryRequest => {
                // now what?

                // If dest is us, reply with response.

                // Else update hop count and add packet to outbound
            }
            BmPacketTypes::RouteDiscoveryResponse => {
                // If dest is us, update table?
                if new_packet.get_destination() == self.stack.get_local_network_id() {
                    self.stack.add_node(
                        BmNodeEntry::new(new_packet.get_originator())
                        .with_route(
                            new_packet.get_source(), 
                            new_packet.get_distance(), 
                            millis, rssi
                        ),
                    );
                }

                // Else update hop count and add packet to outbound
            }
            _ => { }
        }

        // Check if sending node is in our node table
        let node_entry = self.stack.find_node_by_id(new_packet.get_source());
        if node_entry.is_some() {
            // update metrics
            node_entry.unwrap().update_route(
                new_packet.get_source(), 
                new_packet.get_distance(), 
                millis, rssi
            );
        }
        else {
            // add node to table
        }

        // Look up route, modify packet, and queue up outbound

        Some(new_packet)
    }

    pub fn get_next_outbound_packet(&mut self) -> Option<&mut BmNetworkPacket> {
        // Search for a packet that is ok to transmit
        for pkt in self.outbound.iter_mut() {
            if pkt.ok_to_transmit {
                return Some(pkt)
            }
        }
        None
    }

    pub fn set_next_outbound_complete(&mut self, time_millis: i64) {
        // Concern, will the iterator order change if the outbound queue is pushed mid event??
        // Might need to latch an outbound packet here as it cannot be stored in the mesh_task loop.
        for pkt in self.outbound.iter_mut() {
            if pkt.ok_to_transmit {
                // Record timestamp of last tx
                pkt.last_tx_timestamp = time_millis;
                // Increment tx counter
                pkt.tx_count += 1;
                // Remove from list of available packets to tx
                pkt.ok_to_transmit = false;
            }
        }
    }

    pub fn initiate_packet_transfer(&mut self, dest: NetworkId, ack: bool, ttl: u8, payload: BmNetworkPacketPayload) {
        // Check stack if we have route
        let node = self.stack.find_node_by_id(dest);
        if node.is_none() {
            // Start network discovery for destination node
            self.outbound.push(
                BmNetworkPacket::new(
                    BmPacketTypes::RouteDiscoveryRequest, 
                    self.stack.get_local_network_id(), 
                    dest
                ).with_ttl(ttl)
                .is_ok_to_transmit(),
            ).unwrap();
            self.engine_status = BmEngineStatus::PerformingNetworkDiscovery;
        }
        else {
            // Jump right into sending payload
            self.engine_status = BmEngineStatus::SendingPayload;
        } 
      
        // Queue up data payload to send
        self.outbound.push(
            BmNetworkPacket::new_with_payload(
                BmPacketTypes::DataPayload, 
                self.stack.get_local_network_id(), 
                dest,
                Some(payload)
            )
            .with_ttl(ttl)
            .with_ack(ack)
        ).unwrap();
              
    }

    pub fn run_engine(&mut self, current_time_millis: i64) -> BmEngineStatus {
        match self.engine_status {
            BmEngineStatus::PerformingNetworkDiscovery => {
                // Add some sort of time check to retry

                // if retry's go past x then change to no route 

                if self.outbound[0].tx_count > 0 {
                    if current_time_millis - self.outbound[0].last_tx_timestamp > 10000 {    
                        defmt::info!("run_engine: PerformingNetworkDiscovery - timeout");
                        defmt::info!("current_time_millis={}", defmt::Display2Format(&current_time_millis));
                        defmt::info!("last_tx_timestamp={}", defmt::Display2Format(&self.outbound[0].last_tx_timestamp));  
                        self.engine_status = BmEngineStatus::ErrorNoRoute;
                    }
            }             
            }
            BmEngineStatus::RouteFound => {
                defmt::info!("run_engine: RouteFound, sending payload");
                self.engine_status = BmEngineStatus::SendingPayload;

                // update outbound packet with new routing info
            }
            BmEngineStatus::ErrorNoRoute => {
                defmt::info!("run_engine: ErrorNoRoute");
                self.engine_status = BmEngineStatus::Complete;
            }
            _ => { }
        }
        self.engine_status.clone()
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

    
}