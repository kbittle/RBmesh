use heapless::Vec; use crate::BmError;

// fixed capacity `std::Vec`
use super::{
    bm_network_configs::*, bm_network_packet::bm_network_packet::{
        BmNetworkPacket, BmNetworkPacketPayload, BmPacketTypes, TransmitState
    }, bm_network_routing_table::BmNetworkRoutingTable, NetworkId, RssiType, TimeType
};
use defmt::write;

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
        let mut new_packet = BmNetworkPacket::from(length, buffer)?.with_rssi(rssi);

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
        
        // If dest is us, handle packet based off type (accept even if TTL expired)
        let is_for_us = new_packet.get_destination() == self.table.get_local_network_id();
        if is_for_us {
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
                    defmt::info!("rb_engine: Rx Disc Error from {}", new_packet.get_source());
    
                    // Route discovery failed for a destination we were trying to reach
                    if self.engine_status == BmEngineStatus::PerformingNetworkDiscovery {
                        // Check if this error is for our current discovery target
                        if let Some(pkt) = self.get_working_packet() {
                            if pkt.get_destination() == new_packet.get_source() {
                                defmt::info!("rb_engine: Discovery failed for target, transitioning to ErrorNoRoute");
                                self.engine_status = BmEngineStatus::ErrorNoRoute;
                            }
                        }
                    }
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
    } else { // Route packet not addressed to us
        // Check TTL for relay packets
        if new_packet.get_info().hop_count() >= new_packet.get_info().ttl() {
            defmt::warn!("rb_engine: TTL expired, kill packet");
            return None
        }

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
                        // Generate RouteDiscoveryError back to originator
                        self.generate_route_discovery_error(
                            new_packet.get_originator(),
                            new_packet.get_source(),
                            new_packet.get_info().ttl(),
                            new_packet.get_info().required_ack()
                        );
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

    pub fn initiate_packet_transfer(&mut self, dest: NetworkId, ack: bool, ttl: u8, payload: BmNetworkPacketPayload) -> BmError {
        let mut return_value = BmError::None;

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
                return BmError::QueueFull
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
            return_value = BmError::Busy;
        }

        return_value       
    }

    pub fn get_inbound_message_count(&mut self) -> usize {
        self.inbound.len()
    }

    pub fn get_inbound_message(&mut self) -> Option<BmNetworkPacket> {
        self.inbound.pop()
    }

    fn get_working_packet(&mut self) -> Option<&mut BmNetworkPacket> {
        self.working_outbound_index.and_then(|idx| self.outbound.get_mut(idx))
    }

    fn get_working_packet_mut(&mut self) -> Option<&mut BmNetworkPacket> {
        self.working_outbound_index.and_then(|idx| self.outbound.get_mut(idx))
    }

    pub fn run_engine(&mut self, current_time_millis: i64) -> BmEngineStatus {
        let current_engine_status = self.engine_status.clone();
        match current_engine_status {
            BmEngineStatus::PerformingNetworkDiscovery => {
                // Timeout on route discovery
                if let Some(pkt) = self.get_working_packet() {
                    if let Some(tx_comp_time) = pkt.tx_complete_timestamp {
                        if current_time_millis - tx_comp_time > 10000 {    
                            defmt::info!("run_engine: PerformingNetworkDiscovery - timeout");
                            self.engine_status = BmEngineStatus::ErrorNoRoute;
                        }
                    }
                } else {
                    defmt::error!("PerformingNetworkDiscovery: no working packet");
                    self.engine_status = BmEngineStatus::ErrorNoRoute;
                }                     
            }
            BmEngineStatus::RouteFound => {
                // Remove node discovery packet from outbound
                self.clear_working_packet();

                // Find data payload and set working buffer index
                if self.select_data_packet() {
                    defmt::info!("run_engine: RouteFound -> SendingPayload");
                    self.engine_status = BmEngineStatus::SendingPayload;
                }                
                else {
                    defmt::warn!("run_engine: RouteFound -> Complete, could not find data pkt");
                    self.engine_status = BmEngineStatus::Complete;
                } 
            }
            BmEngineStatus::SendingPayload => {
                self.send_data_payload();
            }
            BmEngineStatus::RetryingPayload => {
                defmt::info!("run_engine: RetryingPayload -> SendingPayload");
                
                if let Some(pkt) = self.get_working_packet() {
                    pkt.tx_count += 1;
                    self.engine_status = BmEngineStatus::SendingPayload;
                } else {
                    defmt::error!("RetryingPayload: no working packet");
                    self.engine_status = BmEngineStatus::ErrorNoAck;
                }
            }
            BmEngineStatus::WaitingForAck => {
                if let Some(pkt) = self.get_working_packet() {
                    if let Some(tx_comp_time) = pkt.tx_complete_timestamp {
                        if current_time_millis - tx_comp_time > 10000 {    
                            defmt::info!("run_engine: WaitingForAck -> ErrorNoAck");
                            let next_hop = pkt.get_next_hop();
                            let _ = pkt;
                            self.table.set_node_error(next_hop, current_time_millis);
                            self.engine_status = BmEngineStatus::ErrorNoAck;
                        }
                    }
                } else {
                    defmt::error!("WaitingForAck: no working packet");
                    self.engine_status = BmEngineStatus::ErrorNoAck;
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
                if let Some(pkt) = self.get_working_packet() {
                    if pkt.tx_count < BM_PACKET_RETRY_COUNT {
                        defmt::info!("run_engine: ErrorNoAck -> RetryingPayload");
                        self.engine_status = BmEngineStatus::RetryingPayload;
                    } else {                    
                        defmt::info!("run_engine: ErrorNoAck -> Complete");
                        self.engine_status = BmEngineStatus::Complete;
                    }
                } else {
                    defmt::error!("ErrorNoAck: no working packet");
                    self.engine_status = BmEngineStatus::Complete;
                }
            }
            BmEngineStatus::Complete => {
                if let Some(pkt) = self.get_working_packet() {
                    if !pkt.is_ok_to_transmit() {
                        defmt::info!("run_engine: Complete");
                        self.clear_working_packet();
                        self.engine_status = BmEngineStatus::Idle;
                    }
                } else {
                    defmt::error!("Complete: no working packet");
                    self.engine_status = BmEngineStatus::Idle;
                }
            }
            _ => { }
        }
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
            // Remove working buffer if index is valid
            if index < self.outbound.len() {
                self.outbound.remove(index);
            } else {
                defmt::error!("clear_working_packet: index out of bounds");
            }
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
        if self.outbound.push(packet_to_broadcast).is_err() {
            defmt::error!("broadcast_packet: outbound queue full");
        }
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
            if self.outbound.push(packet_to_route).is_err() {
                defmt::error!("route_packet: outbound queue full");
                return false;
            }
            return true
        }
        false
    }

    fn send_data_payload(&mut self) {
        let dest_id = self.get_working_packet().map(|pkt| pkt.get_destination());
        if let Some(dest_id) = dest_id {
            // Check if we have route to destination
            if let Some(next_hop) = self.table.get_next_hop(dest_id) {
                // Update outbound packet with new next_hop
                if let Some(pkt) = self.get_working_packet_mut() {
                    pkt.set_next_hop(Some(next_hop));
                    pkt.set_wait_for_reply();
                    pkt.set_ok_to_transmit();

                    // Check if Ack is required and transition to next state
                    if pkt.get_info().required_ack() {
                        defmt::info!("run_engine: SendingPayload -> WaitingForAck");
                        self.engine_status = BmEngineStatus::WaitingForAck;
                    }
                    else {                    
                        defmt::info!("run_engine: SendingPayload -> Complete");
                        self.engine_status = BmEngineStatus::Complete;
                    }
                }
            }
            else {
                defmt::warn!("run_engine: SendingPayload -> ErrorNoRoute");
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

    fn generate_route_discovery_error(&mut self, dest: NetworkId, next_hop: NetworkId, ttl: u8, ack: bool) {
        defmt::info!("generate_route_discovery_error: dest={}", dest);
        
        if self.outbound.push(
            BmNetworkPacket::new(
                BmPacketTypes::RouteDiscoveryError,
                self.table.get_local_network_id(),
                next_hop,
                dest,
                ttl,
                ack,
                None
            ).with_ok_to_transmit(),
        ).is_err() {
            defmt::error!("generate_route_discovery_error: outbound queue full");
        }
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;

    // Satisfy defmt linker symbol for host unit tests
    #[no_mangle]
    fn _defmt_timestamp() -> u64 {
        0
    }

    #[test]
    fn test_engine_initialization() {
        // Initialize engine with random network id 
        let mut bm_engine = BmNetworkEngine::new(Some(5));
        
        // Initial state should be Idle and outbound/inbound buffers empty
        assert_eq!(bm_engine.get_next_outbound_packet(), None);
        assert_eq!(bm_engine.get_inbound_message_count(), 0);
        assert_eq!(bm_engine.run_engine(0), BmEngineStatus::Idle);
    }

    #[test]
    fn test_initiate_packet_transfer_without_route_triggers_discovery() {
        let mut bm_engine = BmNetworkEngine::new(Some(1));
        let dest_id = Some(2);
        let payload = BmNetworkPacketPayload::default();

        // Initiating transfer to an unknown route should start network discovery
        let err = bm_engine.initiate_packet_transfer(dest_id, true, 5, payload);
        assert_eq!(err, BmError::None);

        // State machine should transition to PerformingNetworkDiscovery
        assert_eq!(bm_engine.run_engine(0), BmEngineStatus::PerformingNetworkDiscovery);

        // A Route Discovery Request packet should be generated and ready for transmit
        let next_pkt = bm_engine.get_next_outbound_packet();
        assert!(next_pkt.is_some());
        
        let pkt = next_pkt.unwrap();
        assert_eq!(pkt.packet_type, BmPacketTypes::RouteDiscoveryRequest);
        assert_eq!(pkt.get_destination(), dest_id);
    }

    #[test]
    fn test_busy_when_initiating_transfer_twice() {
        let mut bm_engine = BmNetworkEngine::new(Some(1));
        let payload = BmNetworkPacketPayload::default();

        // First transfer succeeds
        let err1 = bm_engine.initiate_packet_transfer(Some(2), true, 5, payload.clone());
        assert_eq!(err1, BmError::None);

        // Second transfer while engine is active should return Busy
        let err2 = bm_engine.initiate_packet_transfer(Some(3), true, 5, payload);
        assert_eq!(err2, BmError::Busy);
    }

    #[test]
    fn test_network_discovery_timeout() {
        let mut bm_engine = BmNetworkEngine::new(Some(1));
        let payload = BmNetworkPacketPayload::default();

        // Initiate transfer
        let _ = bm_engine.initiate_packet_transfer(Some(2), true, 5, payload);
        bm_engine.run_engine(0); // Transition into PerformingNetworkDiscovery

        // Simulate transmit finishing at t = 1000ms
        bm_engine.set_next_outbound_complete(1000);

        // Check state before timeout threshold (< 10000ms elapsed)
        assert_eq!(bm_engine.run_engine(5000), BmEngineStatus::PerformingNetworkDiscovery);

        // Check state at timeout boundary (1000 + 10000 = 11000ms, not exceeded)
        // The condition is > 10000, so at 10000ms elapsed it does NOT transition
        assert_eq!(bm_engine.run_engine(11000), BmEngineStatus::PerformingNetworkDiscovery);
        
        // At 11001ms: elapsed = 10001ms > 10000, transitions to ErrorNoRoute
        // run_engine returns the PREVIOUS status, so this call returns PerformingNetworkDiscovery
        // but internally transitions to ErrorNoRoute
        assert_eq!(bm_engine.run_engine(11001), BmEngineStatus::PerformingNetworkDiscovery);
        
        // Next call returns the new status (ErrorNoRoute)
        assert_eq!(bm_engine.run_engine(11002), BmEngineStatus::ErrorNoRoute);
    }

    #[test]
    fn test_ttl_accepts_packet_addressed_to_us_even_when_expired() {
        let mut engine = BmNetworkEngine::new(Some(1));
        
        // Create a DataPayload packet with hop_count >= ttl (TTL expired)
        let mut expired_pkt = BmNetworkPacket::new(
            BmPacketTypes::DataPayload,
            Some(2),  // originator
            Some(2),  // source
            Some(1),  // destination (us)
            1,        // ttl = 1
            true,     // ack required
            None,
        ).with_rssi(-60);
        // Manually set hop_count to exceed TTL
        expired_pkt.set_hop_count(1); // hop_count == ttl
        
        // Serialize
        let mut bytes = expired_pkt.to_bytes().expect("Serialization failed");
        let len = bytes.len();
        
        // Process the packet - should be accepted because it's addressed to us
        let result = engine.process_packet(len, &mut bytes, 1000, -60);
        
        // Should return the packet (not None)
        assert!(result.is_some(), "Packet addressed to us should be accepted even with expired TTL");
        
        // Should be placed in inbound queue
        assert_eq!(engine.get_inbound_message_count(), 1);
    }

    #[test]
    fn test_ttl_drops_relay_packet_when_expired() {
        let mut engine = BmNetworkEngine::new(Some(1));
        
        // Add a route to destination so we try to relay
        engine.table.update_node_route(Some(3), Some(2), 1, 100, -60);
        
        // Create a DataPayload packet NOT addressed to us, with hop_count >= ttl
        let mut expired_pkt = BmNetworkPacket::new(
            BmPacketTypes::DataPayload,
            Some(2),  // originator
            Some(2),  // source
            Some(3),  // destination (not us)
            1,        // ttl = 1
            true,     // ack required
            None,
        ).with_rssi(-60);
        // Manually set hop_count to exceed TTL
        expired_pkt.set_hop_count(1); // hop_count == ttl
        
        // Serialize
        let mut bytes = expired_pkt.to_bytes().expect("Serialization failed");
        let len = bytes.len();
        
        // Process the packet - should be dropped because TTL expired and not for us
        let result = engine.process_packet(len, &mut bytes, 1000, -60);
        
        // Should return None (dropped)
        assert!(result.is_none(), "Relay packet with expired TTL should be dropped");
        
        // Should NOT be placed in outbound queue for relay
        assert_eq!(engine.get_next_outbound_packet(), None);
    }

    #[test]
    fn test_run_engine_safe_without_working_packet() {
        let mut engine = BmNetworkEngine::new(Some(1));
        
        // Engine starts in Idle with no working packet
        // Calling run_engine should not panic
        let status = engine.run_engine(1000);
        assert_eq!(status, BmEngineStatus::Idle);
        
        // Manually set engine status to a state that requires working packet
        // but don't set working_outbound_index
        engine.engine_status = BmEngineStatus::PerformingNetworkDiscovery;
        
        // run_engine should handle missing working packet gracefully
        let status = engine.run_engine(1000);
        assert_eq!(status, BmEngineStatus::PerformingNetworkDiscovery);
        // Next call should transition to ErrorNoRoute
        let status = engine.run_engine(1000);
        assert_eq!(status, BmEngineStatus::ErrorNoRoute);
    }

    #[test]
    fn test_clear_working_packet_invalid_index() {
        let mut engine = BmNetworkEngine::new(Some(1));
        
        // Call clear_working_packet when index is None - should not panic
        engine.clear_working_packet();
        assert_eq!(engine.working_outbound_index, None);
        
        // Set invalid index (out of bounds)
        engine.working_outbound_index = Some(100);
        engine.clear_working_packet();
        assert_eq!(engine.working_outbound_index, None);
    }

    #[test]
    fn test_route_discovery_error_generated_on_routing_failure() {
        let mut engine = BmNetworkEngine::new(Some(1));
        
        // Create a DataPayload packet NOT addressed to us, no route exists
        let mut pkt = BmNetworkPacket::new(
            BmPacketTypes::DataPayload,
            Some(2),  // originator
            Some(2),  // source
            Some(3),  // destination (no route)
            5,        // ttl = 5
            true,     // ack required
            None,
        ).with_rssi(-60);
        
        let mut bytes = pkt.to_bytes().expect("Serialization failed");
        let len = bytes.len();
        
        // Process the packet - should generate RouteDiscoveryError since no route
        let result = engine.process_packet(len, &mut bytes, 1000, -60);
        
        // Should return the packet
        assert!(result.is_some());
        
        // Should have generated a RouteDiscoveryError in outbound queue
        let outbound_pkt = engine.get_next_outbound_packet();
        assert!(outbound_pkt.is_some());
        assert_eq!(outbound_pkt.unwrap().packet_type, BmPacketTypes::RouteDiscoveryError);
    }

    #[test]
    fn test_route_discovery_error_handled_when_addressed_to_us() {
        let mut engine = BmNetworkEngine::new(Some(1));
        let payload = BmNetworkPacketPayload::default();
        
        // Start a transfer to trigger discovery
        let _ = engine.initiate_packet_transfer(Some(2), true, 5, payload);
        engine.run_engine(0); // -> PerformingNetworkDiscovery
        engine.set_next_outbound_complete(100);
        
        // Now simulate receiving a RouteDiscoveryError for our discovery target
        let mut error_bytes = [0u8; 18];
        error_bytes[0] = BmPacketTypes::RouteDiscoveryError as u8;
        error_bytes[1..5].copy_from_slice(&1u32.to_ne_bytes());  // dest = 1 (us)
        error_bytes[5..9].copy_from_slice(&2u32.to_ne_bytes());  // src = 2
        error_bytes[9..13].copy_from_slice(&2u32.to_ne_bytes()); // next_hop = 2
        error_bytes[13..17].copy_from_slice(&2u32.to_ne_bytes()); // orig = 2
        error_bytes[17] = 0x05; // TTL 5
        
        let _ = engine.process_packet(18, &mut error_bytes, 200, -60);
        
        // Engine status should have transitioned to ErrorNoRoute in process_packet
        // run_engine returns the current status (which is now ErrorNoRoute)
        let status = engine.run_engine(250);
        assert_eq!(status, BmEngineStatus::ErrorNoRoute);
    }

    #[test]
    fn test_ttl_clamped_to_max() {
        // TTL > 7 should be clamped to 7 with warning
        let mut pkt = BmNetworkPacket::new(
            BmPacketTypes::DataPayload,
            Some(1),
            Some(2),
            Some(3),
            10,  // TTL > MAX_TTL_HOP_CNT (7)
            true,
            None,
        );
        
        let info = pkt.get_info();
        assert_eq!(info.ttl(), 7, "TTL should be clamped to MAX_TTL_HOP_CNT");
    }
}
