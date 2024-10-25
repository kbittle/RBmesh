use heapless::Vec; // fixed capacity `std::Vec`
use super::{
    bm_network_configs::*,
    NetworkId,
    bm_network_node::bm_network_node::BmNodeEntry,
};


#[derive(Default, Debug, Clone)]
pub struct BmNetworkStack {
    // Local node network id
    network_id: NetworkId,

    // Node list / routing table
    nodes: Vec<BmNodeEntry, BM_MAX_NET_DEVICES>,
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

    pub fn add_node(&mut self, new_node: BmNodeEntry) {
        self.nodes.push(new_node).unwrap();
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

    
}