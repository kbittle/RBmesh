use heapless::Vec; // fixed capacity `std::Vec`
use super::{
    NetworkId, 
    RssiType,
    BmErrors,
    bm_network_node::bm_network_node::BmNodeEntry as BmNodeEntry,
    bm_network_packet::bm_network_packet::BmPacketTypes as BmPacketTypes,
    bm_network_packet::bm_network_packet::BmNetworkPacket as BmNetworkPacket,
};

// Max number of network id's the device can remember. 
const BM_MAX_NET_DEVICES: usize = 256;

// Outbound queue size. Up this later? What if we hold packets for retries?
const BM_OUTBOUND_QUEUE_SIZE: usize = 5;

#[derive(Default, Debug, Clone)]
pub struct BmNetworkStack {
    // Local node network id
    network_id: NetworkId,

    // Node list - is this a neighbor tablr or a routing table?
    nodes: Vec<BmNodeEntry, BM_MAX_NET_DEVICES>,

    // Out packet buffer
    outbound: Vec<BmNetworkPacket, BM_OUTBOUND_QUEUE_SIZE>,
}

impl BmNetworkStack {
    pub fn process_packet(&mut self, length: u8, buffer: &mut [u8], rssi: RssiType) -> Option<BmNetworkPacket> {
        // Parse packet into struct
        // If we cannot successfully parse packet, return
        let new_packet = BmNetworkPacket::from(length, buffer)?;

        // Do not process our own packets
        if new_packet.hdr.orig == self.network_id {
            return None
        }

        // Handle packet based off type
        match new_packet.hdr.packet_type {
            BmPacketTypes::RouteDiscoveryRequest => {
                // now what?

                // If dest is us, reply with response.

                // Else update hop count and add packet to outbound
            }
            BmPacketTypes::RouteDiscoveryResponse => {

                // If dest is us, update table?

                // Else update hop count and add packet to outbound
            }
            _ => { }
        }

        // Check if sending node is in our node table
        let node_entry = self.find_node_by_id(new_packet.hdr.src);
        if node_entry.is_some() {
            // update metrics
            node_entry.unwrap().update_rssi(rssi);
        }
        else {
            // add node to table
        }

        // Look up route, modify packet, and queue up outbound

        Some(new_packet)
    }

    pub fn get_outbound_packet(&mut self)  -> Option<BmNetworkPacket> {
        self.outbound.pop()
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

    fn find_node_by_id(&mut self, net_id:NetworkId) -> Option<&mut BmNodeEntry> {
        for node in &mut self.nodes {
            if node.dest == net_id {
                return Some(node);
            }
        }
        None
    }
}