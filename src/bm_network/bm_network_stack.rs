use heapless::Vec; // fixed capacity `std::Vec`
use super::{
    bm_network_configs::*,
    NetworkId, TimeType, RssiType,
    bm_network_node::bm_network_node::BmNodeEntry,
};


#[derive(Default, Debug, Clone)]
pub struct BmNetworkStack {
    // Local node network id
    network_id: NetworkId,

    // Node list / routing table
    nodes: Vec<BmNodeEntry, BM_MAX_NET_DEVICES>, // TODO - NodeEntry is ~160bytes, so 16Kb stack consumed
}

impl BmNetworkStack {
    pub fn new(local_network_id: NetworkId) -> Self{
        BmNetworkStack {
            network_id: local_network_id,
            nodes: Vec::new(),
        }
    }

    pub fn get_local_network_id(&mut self) -> NetworkId {
        self.network_id
    }

    pub fn find_node_by_id(&mut self, net_id:NetworkId) -> Option<&mut BmNodeEntry> {
        for node in &mut self.nodes {
            if node.dest_id == net_id {
                return Some(node);
            }
        }
        None
    }

    // Function to add or update nodes and node routes in stack.
    pub fn update_node_route(&mut self, orig_id: NetworkId, next_hop: NetworkId, distance: u8, millis: TimeType, rssi: RssiType) {
        if let Some(node_entry) = self.find_node_by_id(orig_id) {
            // If the node exists, update the route
            node_entry.update_route(
                next_hop, distance, millis, rssi
            );
        }
        else {
            let new_node_entry = BmNodeEntry::new(orig_id).with_route(next_hop, distance, millis, rssi);
            
            defmt::info!("rb_stack: node node={}", defmt::Display2Format(&new_node_entry));

            self.add_node( new_node_entry );
        }
    }

    pub fn get_next_hop(&mut self, dest_id: NetworkId) -> NetworkId {
        // Search through node list for dest node

        // look up primary route

        // return network id

        None
    }

    pub fn add_node(&mut self, new_node: BmNodeEntry) {
        self.nodes.push(new_node).unwrap();
    }

    pub fn get_num_nodes(&mut self) -> usize {
        self.nodes.len()
    }

    pub fn get_node_by_idx(&mut self, index: usize) -> Option<&mut BmNodeEntry> {
        self.nodes.get_mut(index)
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 
    
}