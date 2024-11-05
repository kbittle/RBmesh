use core::ops::Index;

use heapless::Vec; // fixed capacity `std::Vec`
use super::{
    bm_network_configs::*, bm_network_node::bm_network_node::BmNodeEntry, bm_network_packet::bm_network_packet::{
        BmNetworkHdrInfo, BmNetworkPacket, BmNetworkPacketPayload, BmNetworkRoutingHdr, BmPacketTypes, TransmitState
    }, bm_network_routing_table::BmNetworkRoutingTable, NetworkId, RssiType, TimeType
};
use defmt::{write, unwrap};

#[derive(Debug, Default, Clone, PartialEq)]
pub enum BmEngineStatus {
    #[default]
    Idle,
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
            BmEngineStatus::Idle => write!(fmt, "Idle"),
            BmEngineStatus::PerformingNetworkDiscovery => write!(fmt, "PerformingNetworkDiscovery"),
            BmEngineStatus::RouteFound => write!(fmt, "RouteFound"),
            BmEngineStatus::SendingPayload => write!(fmt, "SendingPayload"),
            BmEngineStatus::RetryingPayload => write!(fmt, "RetryingPayload"),
            BmEngineStatus::WaitingForAck => write!(fmt, "WaitingForAck"),
            BmEngineStatus::AckReceieved => write!(fmt, "AckReceieved"),
            BmEngineStatus::ErrorNoRoute => write!(fmt, "ErrorNoRoute"),
            BmEngineStatus::ErrorNoAck => write!(fmt, "ErrorNoAck"),
            BmEngineStatus::Complete => write!(fmt, "Complete"),
        }
    }
}

pub struct BmNetworkEngine {
    pub table: BmNetworkRoutingTable,

    // In packet buffer
    inbound: Vec<BmNetworkPacket, BM_INBOUND_QUEUE_SIZE>,

    // Out packet buffer
    outbound: Vec<BmNetworkPacket, BM_OUTBOUND_QUEUE_SIZE>,

    // Index of packet we are currently handling in the engine state machine
    working_outbound_index: Option<usize>,

    // Enum state machine for status of mesh engine
    engine_status: BmEngineStatus,
}

impl BmNetworkEngine {
    // Constructor
    pub fn new(local_network_id: NetworkId) -> Self {
        BmNetworkEngine {
            table: BmNetworkRoutingTable::new(local_network_id),
            inbound: Vec::new(),
            outbound: Vec::new(),
            working_outbound_index: None,
            engine_status: BmEngineStatus::default(),
        }
    }

    pub fn process_packet(&mut self, length: usize, buffer: &mut [u8], millis: TimeType, rssi: RssiType) -> Option<BmNetworkPacket> {
        // Parse packet into struct
        // If we cannot successfully parse packet, return
        let mut new_packet = BmNetworkPacket::from(length, buffer)?;

        defmt::info!("process_packet len={}", length);

        // Do not process our own packets
        if new_packet.get_originator() == self.table.get_local_network_id() {
            return None
        }

        // Update routing table. Even if the packet is direct and not relayed. We want 
        // the neighbor node to show up as a route with distance 0.
        self.table.update_node_route(
            new_packet.get_originator(), 
            new_packet.get_source(),
            new_packet.get_hop_count(),
            millis, rssi);
        
        // Check hop count against TTL of packet
        // TODO: move this logic into just the packet relay sections?
        //       i.e. if we are the destination at 3 of 3 hops, we should accept
        if new_packet.get_info().hop_count() >= new_packet.get_info().ttl() {
            defmt::warn!("rb_engine: TTL expired, kill packet");
            return None
        }

        // If dest is us, handle packet based off type
        if new_packet.get_destination() == self.table.get_local_network_id() {
            match new_packet.packet_type {
                BmPacketTypes::RouteDiscoveryRequest => {
                    defmt::info!("rb_engine: Rx Disc Req to us, Tx Disc Resp");
    
                    // Queue up discovery response. Addressed to the originator 
                    // through the node we received this from. Same TTL and info bits.
                    if self.outbound.push(
                        BmNetworkPacket::new(
                            BmPacketTypes::RouteDiscoveryResponse, 
                            self.table.get_local_network_id(),
                            new_packet.get_source(),
                            new_packet.get_originator(),
                            new_packet.get_info().ttl(),
                            new_packet.get_info().required_ack(),
                            None
                        )
                        .with_ok_to_transmit(),
                    ).is_err() {
                        defmt::error!("rb_engine: Error queue full");
                    }
                }
                BmPacketTypes::RouteDiscoveryResponse => {    
                    // Discovery Response addressed to us. Theoretically our route is found.
                    if self.engine_status == BmEngineStatus::PerformingNetworkDiscovery {
                        defmt::info!("rb_engine: Rx Disc Resp, route found");
                        self.engine_status = BmEngineStatus::RouteFound;
                    }
                    else {
                        defmt::error!("rb_engine: Rx Disc Resp, unexpected");
                    }
                }
                BmPacketTypes::RouteDiscoveryError => {
                    defmt::info!("rb_engine: Rx Disc Error");
    
                    // What todo with disc error addressed to us??
                }
                BmPacketTypes::DataPayload => {
                    defmt::info!("rb_engine: Rx DataPayload");

                    // Save packet to inbound queue
                    if self.inbound.push(new_packet.clone()).is_err() {
                        defmt::error!("rb_engine: Error in queue full");
                    }

                    // Send ACK response if required
                    if new_packet.get_info().required_ack() {
                        defmt::info!("rb_engine: Rx DataPayload, sending ack");
                        if self.outbound.push(
                            BmNetworkPacket::new(
                                BmPacketTypes::DataPayloadAck, 
                                self.table.get_local_network_id(),
                                new_packet.get_source(),
                                new_packet.get_originator(),
                                new_packet.get_info().ttl(),
                                new_packet.get_info().required_ack(),
                                None
                            )
                            .with_ok_to_transmit(),
                        ).is_err() {
                            defmt::error!("rb_engine: Error queue full");
                        }
                    }
                }
                BmPacketTypes::DataPayloadAck => {
                    if self.engine_status == BmEngineStatus::WaitingForAck {
                        defmt::info!("rb_engine: Rx DataPayloadAck");
                        self.engine_status = BmEngineStatus::AckReceieved;
                    }
                    else {
                        defmt::error!("rb_engine: Rx DataPayloadAck, unexpected");
                    }
                }
                BmPacketTypes::BcastNeighborTable => {
                    defmt::info!("rb_engine: Rx Neighbor table");
                    // Should never receieve addressed neighbor table packet
                }
            }
        }
        else { // Route packet not addressed to us
            match new_packet.packet_type {
                BmPacketTypes::RouteDiscoveryRequest |
                BmPacketTypes::BcastNeighborTable => {
                    defmt::info!("rb_engine: rebroadcast packet");

                    self.broadcast_packet(new_packet.clone());
                }
                BmPacketTypes::RouteDiscoveryResponse |
                BmPacketTypes::DataPayload |
                BmPacketTypes::DataPayloadAck  => {
                    defmt::info!("rb_engine: routing packet");
    
                    if !self.route_packet(new_packet.clone()) {
                        // Generate discovery error??
                    } 
                }
                _ => {}
            }
        }

        Some(new_packet)
    }

    // Function to search for next outbound packet that is available to transmit.
    pub fn get_next_outbound_packet(&mut self) -> Option<&mut BmNetworkPacket> {
        // Search for a packet that is ok to transmit
        for pkt in self.outbound.iter_mut() {
            if pkt.is_ok_to_transmit() {
                return Some(pkt)
            }
        }
        None
    }

    pub fn set_next_outbound_complete(&mut self, time_millis: i64) {
        // Concern, will the iterator order change if the outbound queue is pushed mid event??
        // Might need to latch an outbound packet here as it cannot be stored in the mesh_task loop.
        for (index, pkt) in self.outbound.iter_mut().enumerate() {
            if pkt.is_ok_to_transmit() {
                if pkt.is_waiting_for_reply() {
                    // Record timestamp of last tx
                    pkt.tx_complete_timestamp = Some(time_millis);
                    // Increment tx counter
                    pkt.tx_count += 1;
                    // Remove from list of available packets to tx
                    pkt.tx_state = TransmitState::Complete;
                }
                else {
                    // If state machine is not waiting for a resp, remove successfully transmitted packet.
                    self.outbound.remove(index);
                }
                return
            }           
        }
    }

    pub fn initiate_packet_transfer(&mut self, dest: NetworkId, ack: bool, ttl: u8, payload: BmNetworkPacketPayload) {
        if self.engine_status == BmEngineStatus::Idle {
            // Queue up data payload to send
            if self.outbound.push(
                BmNetworkPacket::new(
                    BmPacketTypes::DataPayload, 
                    self.table.get_local_network_id(),
                    None,
                    dest,
                    ttl,
                    ack,
                    Some(payload)
                ).with_wait_for_reply()
            ).is_err() {
                defmt::error!("Error queue full");
                return
            }

            // Check stack if we have route
            if self.table.find_node_by_id(dest).is_none() {
                // Start network discovery for destination node
                self.start_network_discovery(dest, ttl);
            }
            else {
                // Set data packet as working packet
                self.select_data_packet();
                // Jump right into sending payload
                self.engine_status = BmEngineStatus::SendingPayload;
            }           
        }
        else {
            defmt::warn!("initiate_packet_transfer: busy");
        }            
    }

    pub fn get_inbound_message_count(&mut self) -> usize {
        self.inbound.len()
    }

    pub fn get_inbound_message(&mut self) -> Option<BmNetworkPacket> {
        self.inbound.pop()
    }

    pub fn run_engine(&mut self, current_time_millis: i64) -> BmEngineStatus {
        let current_engine_status = self.engine_status.clone();
        match current_engine_status {
            BmEngineStatus::PerformingNetworkDiscovery => {
                // TODO - Add some sort of time check to retry?

                // Timeout on route discovery
                // TODO - make timeout dynamic off the hop count and radio settings
                // TODO - currently timeout includes tx time + rx time. Maybe change so timeout doesnt start until tx complete
                if let Some(tx_comp_time) = self.outbound[self.working_outbound_index.unwrap()].tx_complete_timestamp {
                    if current_time_millis - tx_comp_time > 10000 {    
                        defmt::info!("run_engine: PerformingNetworkDiscovery - timeout");
                        defmt::info!("current_time_millis={}", defmt::Display2Format(&current_time_millis));
                        defmt::info!("tx_complete_timestamp={}", defmt::Display2Format(&tx_comp_time));  
                        self.engine_status = BmEngineStatus::ErrorNoRoute;
                    }
                }                     
            }
            BmEngineStatus::RouteFound => {
                // Remove node discovery packet from outbound
                self.clear_working_packet();

                // Find data payload and set working buffer index
                if self.select_data_packet() {
                    defmt::info!("run_engine: RouteFound -> SendingPayload");

                    // Transition to send payload
                    self.engine_status = BmEngineStatus::SendingPayload;
                }                
                else {
                    defmt::warn!("run_engine: RouteFound -> Complete, could not find data pkt");

                    // Transition to complete
                    self.engine_status = BmEngineStatus::Complete;
                } 
            }
            BmEngineStatus::SendingPayload => {
                self.send_data_payload();
            }
            BmEngineStatus::RetryingPayload => {
                defmt::info!("run_engine: RetryingPayload -> SendingPayload");
                
                self.outbound[self.working_outbound_index.unwrap()].tx_count += 1;

                // TODO - anything else to update?

                // Transition to send payload which will search for the best route
                self.engine_status = BmEngineStatus::SendingPayload;
            }
            BmEngineStatus::WaitingForAck => {
                // Handle timeout on data payload
                // TODO - currently timeout includes tx time + rx time. Maybe change so timeout doesnt start until tx complete
                if let Some(tx_comp_time) = self.outbound[self.working_outbound_index.unwrap()].tx_complete_timestamp {
                    if current_time_millis - tx_comp_time > 10000 {    
                        defmt::info!("run_engine: WaitingForAck - timeout");
                        defmt::info!("current_time_millis={}", defmt::Display2Format(&current_time_millis));
                        defmt::info!("tx_complete_timestamp={}", defmt::Display2Format(&tx_comp_time));  
    
                        // Record error on that route
                        self.table.set_node_error(
                            self.outbound[self.working_outbound_index.unwrap()].get_next_hop(), 
                            current_time_millis);
                        
                        // Check if tx count is below threshold
                        if self.outbound[self.working_outbound_index.unwrap()].tx_count < BM_PACKET_RETRY_COUNT {
                            self.engine_status = BmEngineStatus::RetryingPayload;
                        }
                        else {
                            self.engine_status = BmEngineStatus::ErrorNoAck;
                        }
                    }
                }                
            }
            BmEngineStatus::AckReceieved => {
                defmt::info!("run_engine: AckReceieved -> Complete");

                self.engine_status = BmEngineStatus::Complete;
            }
            BmEngineStatus::ErrorNoRoute => {
                defmt::info!("run_engine: ErrorNoRoute -> Complete");

                self.engine_status = BmEngineStatus::Complete;
            }
            BmEngineStatus::ErrorNoAck => {
                defmt::info!("run_engine: ErrorNoAck -> Complete");

                // TODO -  add support for retransmits. Need to record error on that route and re-evaluate if there is a better route.

                self.engine_status = BmEngineStatus::Complete;

            }
            BmEngineStatus::Complete => {
                // Wait for transmit to complete before erasing working packet
                if !self.outbound[self.working_outbound_index.unwrap()].is_ok_to_transmit() {
                    defmt::info!("run_engine: Complete");

                    self.clear_working_packet();

                    self.engine_status = BmEngineStatus::Idle;
                }
            }
            _ => { }
        }
        // Return the current engine status, not the new status
        current_engine_status
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

    fn start_network_discovery(&mut self, dest: NetworkId, ttl: u8) {
        defmt::info!("start_network_discovery: id={}", dest);

        if self.outbound.push(
            BmNetworkPacket::new(
                BmPacketTypes::RouteDiscoveryRequest, 
                self.table.get_local_network_id(),
                None,
                dest,
                ttl,
                false,
                None
            ).with_ok_to_transmit()
            .with_wait_for_reply(),
        ).is_err() {
            defmt::error!("Error queue full");
        }

        self.working_outbound_index = Some(self.outbound.len() - 1);
        self.engine_status = BmEngineStatus::PerformingNetworkDiscovery;
    }

    fn clear_working_packet(&mut self) {
        if let Some(index) = self.working_outbound_index {
            // Remove working buffer
            self.outbound.remove(index);
            // Invalidate working buff index
            self.working_outbound_index = None;
        }
        else {
            defmt::error!("No working index");
        }
    }

    fn broadcast_packet(&mut self, mut packet_to_broadcast: BmNetworkPacket) {
        // Update source with our network id
        packet_to_broadcast.set_source(self.table.get_local_network_id());
        // Increment hop count
        packet_to_broadcast.increment_hop_count();
        // Set Ok to transmit
        packet_to_broadcast.set_ok_to_transmit();
        // Push updated packet to outbound queue
        self.outbound.push(packet_to_broadcast).unwrap();
    }

    fn route_packet(&mut self, mut packet_to_route: BmNetworkPacket) -> bool {
        // Check if we have route to destination
        if let Some(next_hop) = self.table.get_next_hop(packet_to_route.get_destination()) {
            // Update source with our network id
            packet_to_route.set_source(self.table.get_local_network_id());
            // Increment hop count
            packet_to_route.increment_hop_count();
            // Update next_hop from routing table
            packet_to_route.set_next_hop(Some(next_hop));
            // Set Ok to transmit
            packet_to_route.set_ok_to_transmit();
            // Push updated packet to outbound queue
            self.outbound.push(packet_to_route).unwrap();
            return true
        }
        false
    }

    fn send_data_payload(&mut self) {
        if let Some(working_index) = self.working_outbound_index {
            let dest_id = self.outbound[working_index].get_destination();

            // Check if we have route to destination
            if let Some(next_hop) = self.table.get_next_hop(dest_id) {
                // Update outbound packet with new next_hop
                self.outbound[working_index].set_next_hop(Some(next_hop));

                // Mark packet as ok to transmit
                self.outbound[working_index].set_wait_for_reply();
                self.outbound[working_index].set_ok_to_transmit();

                // Check if Ack is required and transition to next state
                if self.outbound[working_index].get_info().required_ack() {
                    defmt::info!("run_engine: SendingPayload -> WaitingForAck");
                    self.engine_status = BmEngineStatus::WaitingForAck;
                }
                else {                    
                    defmt::info!("run_engine: SendingPayload -> Complete");
                    self.engine_status = BmEngineStatus::Complete;
                }
            }
            else {
                defmt::warn!("run_engine: SendingPayload -> ErrorNoRoute");

                // Transition to complete
                self.engine_status = BmEngineStatus::ErrorNoRoute; 
            }
        }
        else {
            defmt::error!("run_engine: SendingPayload, no working index");
        }
    }

    // Function to find the data payload packet in the outbound queue. Save that index
    fn select_data_packet(&mut self) -> bool {
        for (index, pkt) in self.outbound.iter_mut().enumerate() {
            if pkt.is_ok_to_transmit() == false && 
               pkt.tx_count == 0 &&
               pkt.packet_type == BmPacketTypes::DataPayload {
                self.working_outbound_index = Some(index);
                return true
            }
        }
        false
    }
    
}