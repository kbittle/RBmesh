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
