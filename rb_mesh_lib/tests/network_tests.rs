use bm_network::{
    bm_network_engine::{BmEngineStatus, BmNetworkEngine},
    bm_network_packet::bm_network_packet::{
        BmNetworkPacketPayload, BmPacketTypes,
    },
    BmError,
};
use std::println;

#[no_mangle]
fn _defmt_timestamp() -> u64 {
    0
}

/// # Single-Node Route Discovery & Packet Completion Cycle
///
/// **Scenario:** End-to-end state machine transitions and packet queue updates for a single node 
/// initiating an unrouted transfer to a direct neighbor.
///
/// **Workflow Tested:**
/// 1. **Transfer Initiation:** Initial transfer request queues both a `DataPayload` (blocked) and a `RouteDiscoveryRequest` (ok to transmit).
/// 2. **State Transition (Discovery):** Engine moves to `PerformingNetworkDiscovery` and transmits the discovery request.
/// 3. **Response Processing:** Engine receives a `RouteDiscoveryResponse`, populates the target node route into `BmNetworkRoutingTable`, and transitions state to `RouteFound`.
/// 4. **Payload Selection & Transmit:** Engine transitions `RouteFound` -> `SendingPayload`, unblocks the queued `DataPayload` packet, and marks it for transmission.
/// 5. **Ack Handling & Cleanup:** Engine receives `DataPayloadAck`, transitions `WaitingForAck` -> `AckReceived` -> `Complete` -> `Idle`, and purges completed state.
#[test]
fn test_route_discovery_and_packet_completion_flow() {
    println!("\n=== START: Route Discovery & Packet Completion Test ===");
    
    let node1_id = Some(1);
    let node2_id = Some(2);

    let mut engine = BmNetworkEngine::new(node1_id);
    let payload = BmNetworkPacketPayload::default();

    println!("[INFO] Node 1 Engine initialized. Local ID: {:?}", node1_id);

    // ------------------------------------------------------------------------
    // Step 1: Initiate Transfer to unknown route (Node 2)
    // ------------------------------------------------------------------------
    println!("\n--- Step 1: Initiating transfer to Node 2 (No route exists) ---");
    let err = engine.initiate_packet_transfer(node2_id, true, 5, payload);
    assert_eq!(err, BmError::None);

    let status = engine.run_engine(0);
    println!("[STATE] Engine Status: {:?}", status);
    assert_eq!(status, BmEngineStatus::PerformingNetworkDiscovery);

    // Verify a RouteDiscoveryRequest is ready to transmit
    let discovery_pkt = engine.get_next_outbound_packet().expect("Expected outbound discovery packet");
    
    // Evaluate dest before referencing packet_type immutably
    let dest = discovery_pkt.get_destination();
    println!("[TX QUEUE] Next packet to transmit: Type={:?}, Dest={:?}", 
        discovery_pkt.packet_type, dest
    );
    assert_eq!(discovery_pkt.packet_type, BmPacketTypes::RouteDiscoveryRequest);
    assert_eq!(dest, node2_id);

    // ------------------------------------------------------------------------
    // Step 2: Complete Discovery Request Transmit
    // ------------------------------------------------------------------------
    println!("\n--- Step 2: Completing Discovery Request transmit at t=100ms ---");
    engine.set_next_outbound_complete(100);

    let status = engine.run_engine(150);
    println!("[STATE] Engine Status: {:?}", status);
    assert_eq!(status, BmEngineStatus::PerformingNetworkDiscovery);

    // ------------------------------------------------------------------------
    // Step 3: Simulate receiving RouteDiscoveryResponse from Node 2
    // ------------------------------------------------------------------------
    println!("\n--- Step 3: Simulating Rx RouteDiscoveryResponse from Node 2 ---");
    
    let mut disc_resp_bytes = [0u8; 18];
    disc_resp_bytes[0] = BmPacketTypes::RouteDiscoveryResponse as u8;
    disc_resp_bytes[1..5].copy_from_slice(&1u32.to_ne_bytes());
    disc_resp_bytes[5..9].copy_from_slice(&2u32.to_ne_bytes());
    disc_resp_bytes[9..13].copy_from_slice(&2u32.to_ne_bytes());
    disc_resp_bytes[13..17].copy_from_slice(&2u32.to_ne_bytes());
    disc_resp_bytes[17] = 0x05; // TTL 5

    let processed = engine.process_packet(18, &mut disc_resp_bytes, 200, -60);
    assert!(processed.is_some());
    println!("[RX] Processed Discovery Response from Node 2.");

    let next_hop = engine.table.get_next_hop(node2_id);
    println!("[ROUTING TABLE] Next hop to Node 2 is: {:?}", next_hop);
    assert_eq!(next_hop, node2_id);

    let status = engine.run_engine(250);
    println!("[STATE] Engine Status: {:?}", status);
    assert_eq!(status, BmEngineStatus::RouteFound);

    // ------------------------------------------------------------------------
    // Step 4: Engine selects queued DataPayload and prepares to send
    // ------------------------------------------------------------------------
    println!("\n--- Step 4: Transitioning RouteFound -> SendingPayload ---");
    let status = engine.run_engine(300);
    println!("[STATE] Engine Status: {:?}", status);
    assert_eq!(status, BmEngineStatus::SendingPayload);

    let data_pkt = engine.get_next_outbound_packet().expect("Expected outbound data packet");
    
    // Evaluate next_hop before referencing packet_type immutably
    let next_hop_val = data_pkt.get_next_hop();
    println!("[TX QUEUE] Next packet to transmit: Type={:?}, NextHop={:?}", 
        data_pkt.packet_type, next_hop_val
    );
    assert_eq!(data_pkt.packet_type, BmPacketTypes::DataPayload);
    assert_eq!(next_hop_val, node2_id);

    // ------------------------------------------------------------------------
    // Step 5: Transmit Data Payload
    // ------------------------------------------------------------------------
    println!("\n--- Step 5: Completing Data Payload transmit at t=350ms ---");
    engine.set_next_outbound_complete(350);

    let status = engine.run_engine(400);
    println!("[STATE] Engine Status: {:?}", status);
    assert_eq!(status, BmEngineStatus::WaitingForAck);

    // ------------------------------------------------------------------------
    // Step 6: Simulate receiving DataPayloadAck from Node 2
    // ------------------------------------------------------------------------
    println!("\n--- Step 6: Simulating Rx DataPayloadAck from Node 2 ---");
    let mut ack_bytes = [0u8; 18];
    ack_bytes[0] = BmPacketTypes::DataPayloadAck as u8;
    ack_bytes[1..5].copy_from_slice(&1u32.to_ne_bytes());
    ack_bytes[5..9].copy_from_slice(&2u32.to_ne_bytes());
    ack_bytes[9..13].copy_from_slice(&2u32.to_ne_bytes());
    ack_bytes[13..17].copy_from_slice(&2u32.to_ne_bytes());
    ack_bytes[17] = 0x05;

    let ack_processed = engine.process_packet(18, &mut ack_bytes, 450, -55);
    assert!(ack_processed.is_some());
    println!("[RX] Processed DataPayloadAck from Node 2.");

    let status1 = engine.run_engine(500);
    println!("[STATE] Engine Status: {:?}", status1);
    assert_eq!(status1, BmEngineStatus::AckReceieved);

    let status2 = engine.run_engine(550);
    println!("[STATE] Engine Status: {:?}", status2);
    assert_eq!(status2, BmEngineStatus::Complete);
    
    let status3 = engine.run_engine(600);
    println!("[STATE] Engine Status: {:?}", status3);
    assert_eq!(status3, BmEngineStatus::Idle);

    println!("\n=== END: Test Passed Successfully! ===");
}

/// # Multi-Hop Packet Routing & Forwarding Test
///
/// **Scenario:** Multi-hop packet traversal across a 3-node linear topology (`Node 1 <-> Node 2 <-> Node 3`).
///
/// **Workflow Tested:**
/// 1. **Route Discovery Trigger:** Node 1 attempts to transmit to Node 3 without an existing route, queuing and broadcasting a `RouteDiscoveryRequest`.
/// 2. **Request Forwarding:** Node 2 intercepts the broadcast request, increments the hop count, updates the source field, and forwards it to Node 3.
/// 3. **Response Generation & Route Learning:** Node 3 processes the request, learns the reverse route back to Node 1 via Node 2, and generates a `RouteDiscoveryResponse`.
/// 4. **Response Forwarding:** Node 2 forwards the discovery response back to Node 1.
/// 5. **Payload Route Resolution:** Node 1 receives the response, resolves its routing table entry for Node 3 via Node 2, and transmits the pending `DataPayload`.
/// 6. **Payload Forwarding:** Node 2 intercepts the payload packet and successfully queues it for forwarding to its final destination (Node 3).
#[test]
fn test_packet_routing() {
    println!("\n=== START: Multi-Hop Packet Routing & Forwarding Test ===");

    // Define a 3-node linear topology: Node 1 <-> Node 2 <-> Node 3
    let node1_id = Some(1);
    let node2_id = Some(2);
    let node3_id = Some(3);

    let mut engine_node1 = BmNetworkEngine::new(node1_id);
    let mut engine_node2 = BmNetworkEngine::new(node2_id);
    let mut engine_node3 = BmNetworkEngine::new(node3_id);

    // Give Node 2 direct routes to its immediate neighbors (Node 1 and Node 3)
    engine_node2.table.update_node_route(node1_id, node1_id, 0, 100, -50);
    engine_node2.table.update_node_route(node3_id, node3_id, 0, 100, -50);

    // Give Node 3 a direct route back to Node 2
    engine_node3.table.update_node_route(node2_id, node2_id, 0, 100, -50);

    // ------------------------------------------------------------------------
    // Step 1: Node 1 initiates transfer to Node 3 (No direct route)
    // ------------------------------------------------------------------------
    println!("\n--- Step 1: Node 1 initiates discovery for Node 3 ---");
    let payload = BmNetworkPacketPayload::default();
    let err = engine_node1.initiate_packet_transfer(node3_id, true, 5, payload);
    assert_eq!(err, BmError::None);

    assert_eq!(
        engine_node1.run_engine(0),
        BmEngineStatus::PerformingNetworkDiscovery
    );

    // Get Node 1's broadcast discovery request
    let node1_disc_pkt = engine_node1
        .get_next_outbound_packet()
        .expect("Node 1 should have an outbound discovery request");
    
    assert_eq!(node1_disc_pkt.packet_type, BmPacketTypes::RouteDiscoveryRequest);

    // Serialize Node 1's packet to raw OTA bytes
    let mut raw_bytes = node1_disc_pkt.to_bytes().expect("Serialization failed");

    // ------------------------------------------------------------------------
    // Step 2: Node 2 receives & forwards RouteDiscoveryRequest
    // ------------------------------------------------------------------------
    println!("\n--- Step 2: Node 2 receives RouteDiscoveryRequest and forwards it ---");
    let len = raw_bytes.len();
    let processed_at_node2 = engine_node2.process_packet(len, &mut raw_bytes, 100, -60);
    assert!(processed_at_node2.is_some());

    // Pop the forwarded packet using get_next_outbound_packet
    let node2_fwd_pkt = engine_node2
        .get_next_outbound_packet()
        .expect("Node 2 should have queued a forwarded packet");

    // Check that Node 2 updated headers (hop count incremented, src set to Node 2)
    assert_eq!(node2_fwd_pkt.get_source(), node2_id);
    assert_eq!(node2_fwd_pkt.get_originator(), node1_id);
    assert_eq!(node2_fwd_pkt.get_destination(), node3_id);
    assert_eq!(node2_fwd_pkt.get_hop_count(), 1);

    let mut fwd_disc_bytes = node2_fwd_pkt.to_bytes().expect("Serialization failed");
    engine_node2.set_next_outbound_complete(150);

    // ------------------------------------------------------------------------
    // Step 3: Node 3 receives Discovery Request & generates Response
    // ------------------------------------------------------------------------
    println!("\n--- Step 3: Node 3 receives discovery request and queues response ---");
    let len = fwd_disc_bytes.len();
    let processed_at_node3 = engine_node3.process_packet(len, &mut fwd_disc_bytes, 200, -65);
    assert!(processed_at_node3.is_some());

    // Node 3 should learn the route back to Node 1 (via Node 2)
    assert_eq!(engine_node3.table.get_next_hop(node1_id), node2_id);

    // Verify Node 3 created an outbound RouteDiscoveryResponse packet
    let node3_resp_pkt = engine_node3
        .get_next_outbound_packet()
        .expect("Node 3 should have an outbound discovery response");

    assert_eq!(node3_resp_pkt.packet_type, BmPacketTypes::RouteDiscoveryResponse);
    assert_eq!(node3_resp_pkt.get_destination(), node1_id);
    assert_eq!(node3_resp_pkt.get_next_hop(), node2_id);

    let mut resp_bytes = node3_resp_pkt.to_bytes().expect("Serialization failed");
    engine_node3.set_next_outbound_complete(250);

    // ------------------------------------------------------------------------
    // Step 4: Node 2 forwards RouteDiscoveryResponse back to Node 1
    // ------------------------------------------------------------------------
    println!("\n--- Step 4: Node 2 forwards RouteDiscoveryResponse back to Node 1 ---");
    let len = resp_bytes.len();
    let processed_at_node2_resp = engine_node2.process_packet(len, &mut resp_bytes, 300, -60);
    assert!(processed_at_node2_resp.is_some());

    let node2_fwd_resp = engine_node2
        .get_next_outbound_packet()
        .expect("Node 2 should have queued forwarded response");
    assert_eq!(node2_fwd_resp.get_next_hop(), node1_id);

    let mut fwd_resp_bytes = node2_fwd_resp.to_bytes().expect("Serialization failed");
    engine_node2.set_next_outbound_complete(350);

    // ------------------------------------------------------------------------
    // Step 5: Node 1 processes Response and sends Data Payload via Node 2
    // ------------------------------------------------------------------------
    println!("\n--- Step 5: Node 1 processes discovery response and transmits DataPayload ---");
    let len = fwd_resp_bytes.len();
    let _ = engine_node1.process_packet(len, &mut fwd_resp_bytes, 400, -55);

    // Node 1 should now have a valid route to Node 3 via Node 2
    assert_eq!(engine_node1.table.get_next_hop(node3_id), node2_id);

    // Run Node 1 engine cycles
    assert_eq!(engine_node1.run_engine(450), BmEngineStatus::RouteFound);
    assert_eq!(engine_node1.run_engine(500), BmEngineStatus::SendingPayload);

    let data_pkt = engine_node1
        .get_next_outbound_packet()
        .expect("Node 1 should have a queued DataPayload packet");

    assert_eq!(data_pkt.packet_type, BmPacketTypes::DataPayload);
    assert_eq!(data_pkt.get_destination(), node3_id);
    assert_eq!(data_pkt.get_next_hop(), node2_id);

    let mut data_bytes = data_pkt.to_bytes().expect("Serialization failed");
    engine_node1.set_next_outbound_complete(550);

    // ------------------------------------------------------------------------
    // Step 6: Node 2 forwards DataPayload to Node 3
    // ------------------------------------------------------------------------
    println!("\n--- Step 6: Node 2 forwards DataPayload to destination Node 3 ---");
    let len = data_bytes.len();
    let _ = engine_node2.process_packet(len, &mut data_bytes, 600, -60);

    let fwd_data_pkt = engine_node2
        .get_next_outbound_packet()
        .expect("Node 2 should have queued forwarded DataPayload");
    assert_eq!(fwd_data_pkt.packet_type, BmPacketTypes::DataPayload);
    assert_eq!(fwd_data_pkt.get_destination(), node3_id);

    println!("\n=== END: Multi-Hop Routing Test Passed! ===");
}
