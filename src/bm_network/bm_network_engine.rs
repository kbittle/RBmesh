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

        // Update routing table. Even if the packet is direct and not relayed. We want 
        // the neighbor node to show up as a route with distance 0.
        self.stack.update_node_route(
            new_packet.get_originator(), 
            new_packet.get_source(),
            new_packet.get_distance(), 
            millis, rssi);
        
        // Check hop count against TTL of packet
        // TODO: move this logic into just the packet relay sections?
        //       i.e. if we are the destination at 3 of 3 hops, we should accept
        if new_packet.get_info().hop_count() >= new_packet.get_info().ttl() {
            defmt::warn!("rb_engine: TTL expired, kill packet");
            return None
        }

        // Handle packet based off type
        match new_packet.packet_type {
            BmPacketTypes::RouteDiscoveryRequest => {
                // If dest is us, reply with response.
                if new_packet.get_destination() == self.stack.get_local_network_id() {
                    defmt::info!("rb_engine: Rx Disc Req to us, Tx Disc Resp");

                    // Queue up discovery response. Addressed to the originator 
                    // through the node we received this from. Same TTL and info bits.
                    self.outbound.push(
                        BmNetworkPacket::new_with_payload(
                            BmPacketTypes::RouteDiscoveryResponse, 
                            self.stack.get_local_network_id(), 
                            new_packet.get_originator(),
                            None
                        )
                        .with_next_hop(new_packet.get_source())
                        .with_ttl(new_packet.get_info().ttl())
                        .with_ack(new_packet.get_info().required_ack())
                        .is_ok_to_transmit(),
                    ).unwrap();
                } // Else update hop count and add packet to outbound
                else {
                    defmt::info!("rb_engine: Rx Disc Req, relay");

                    // Update source with our network id
                    new_packet.set_source(self.stack.get_local_network_id());
                    // Increment hop count
                    new_packet.increment_hop_count();
                    // Set Ok to transmit
                    new_packet.ok_to_transmit = true;
                    // Push updated packet to outbound queue
                    self.outbound.push(new_packet.clone()).unwrap();
                }
            }
            BmPacketTypes::RouteDiscoveryResponse => {
                // If dest is us, update node table
                if new_packet.get_destination() == self.stack.get_local_network_id() {
                    defmt::info!("rb_engine: Rx Disc Resp to us, update routing table");
                    self.stack.add_node(
                        BmNodeEntry::new(new_packet.get_originator())
                        .with_metrics(millis)
                        .with_route(
                            new_packet.get_source(), 
                            new_packet.get_distance(), 
                            millis, rssi
                        ),
                    );

                    // TODO Look in outbound queue for packets to update next hop

                } // Else update hop count and add packet to outbound
                else {
                    defmt::info!("rb_engine: Rx Disc Resp, relay");

                    // Update source with our network id
                    new_packet.set_source(self.stack.get_local_network_id());
                    // Increment hop count
                    new_packet.increment_hop_count();
                    // Check if we have route to destination
                    if let Some(next_hop) = self.stack.get_next_hop(new_packet.get_destination()) {
                        // Update next_hop from routing table
                        new_packet.set_next_hop(Some(next_hop));
                        // Set Ok to transmit
                        new_packet.ok_to_transmit = true;
                        // Push updated packet to outbound queue
                        self.outbound.push(new_packet.clone()).unwrap();
                    }
                    // Else generate discovery error??
                }
            }
            BmPacketTypes::RouteDiscoveryError => {
                defmt::info!("rb_engine: Rx Disc Error");

                // Not sure if I need this??
            }
            BmPacketTypes::DataPayload => {
                defmt::info!("rb_engine: Rx Data payload");
            }
            BmPacketTypes::DataPayloadAck => {
                defmt::info!("rb_engine: Rx Data payload ack");
            }
            BmPacketTypes::BcastNeighborTable => {
                defmt::info!("rb_engine: Rx Neighbor table");
            }
            _ => {
                defmt::info!("rb_engine: Unknown packet type");
            }
        }        

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
        if self.stack.find_node_by_id(dest).is_none() {
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