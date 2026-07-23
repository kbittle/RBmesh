use heapless::Vec; // fixed capacity `std::Vec`
use super::{
    bm_network_configs::*,
    NetworkId, TimeType, RssiType,
    bm_network_node::bm_network_node::BmNodeEntry,
};
use core::option::Option::{self, Some, None};

#[derive(Default, Debug, Clone)]
pub struct BmNetworkRoutingTable {
    // Local node network id
    network_id: NetworkId,

    // Node list / routing table
    nodes: Vec<BmNodeEntry, BM_MAX_NET_DEVICES>, // TODO - NodeEntry is ~160bytes, so 16Kb stack consumed
}

impl BmNetworkRoutingTable {
    pub fn new(local_network_id: NetworkId) -> Self{
        BmNetworkRoutingTable {
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

    pub fn set_node_error(&mut self, dest_id: NetworkId, millis: TimeType) {
        if let Some(node_entry) = self.find_node_by_id(dest_id) {
            // If the node exists, update the route
            node_entry.record_error(millis);
        }
        else {
            defmt::error!("rb_stack: could not find node");
        }
    }

    pub fn get_next_hop(&mut self, dest_id: NetworkId) -> NetworkId {
        // Search through node list for dest node
        if let Some(node_entry) = self.find_node_by_id(dest_id) {
            // Get best route
            if let Some(mut route) = node_entry.get_best_route() {                
                // Return network id
                return route.get_next_hop()
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_table_initialization() {
        let local_id = Some(1);
        let mut table = BmNetworkRoutingTable::new(local_id);

        assert_eq!(table.get_local_network_id(), local_id);
        assert_eq!(table.get_num_nodes(), 0);
        assert_eq!(table.get_next_hop(Some(2)), None);
    }

    #[test]
    fn test_add_and_find_node() {
        let mut table = BmNetworkRoutingTable::new(Some(1));
        let node_id = Some(2);

        // Initially node should not exist
        assert!(table.find_node_by_id(node_id).is_none());

        // Add a node entry manually
        let new_node = BmNodeEntry::new(node_id);
        table.add_node(new_node);

        // Verify count and lookup
        assert_eq!(table.get_num_nodes(), 1);
        let found = table.find_node_by_id(node_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().dest_id, node_id);
    }

    #[test]
    fn test_update_node_route_creates_and_updates() {
        let mut table = BmNetworkRoutingTable::new(Some(1));
        let orig_id = Some(10);
        let next_hop = Some(20);

        // 1. Updating a non-existent node should create it automatically
        table.update_node_route(orig_id, next_hop, 1, 1000, -65);
        assert_eq!(table.get_num_nodes(), 1);

        // Verify next hop lookups
        assert_eq!(table.get_next_hop(orig_id), next_hop);

        // 2. Updating the same node with new metrics should update existing entry
        let new_next_hop = Some(30);
        table.update_node_route(orig_id, new_next_hop, 0, 2000, -50);

        // Node count should remain 1
        assert_eq!(table.get_num_nodes(), 1);

        // Lookup node directly to verify route updated
        let node = table.find_node_by_id(orig_id).unwrap();
        assert_eq!(node.dest_id, orig_id);
    }

    #[test]
    fn test_get_node_by_idx() {
        let mut table = BmNetworkRoutingTable::new(Some(1));

        table.add_node(BmNodeEntry::new(Some(100)));
        table.add_node(BmNodeEntry::new(Some(200)));

        assert_eq!(table.get_num_nodes(), 2);

        // Valid indices
        assert_eq!(table.get_node_by_idx(0).unwrap().dest_id, Some(100));
        assert_eq!(table.get_node_by_idx(1).unwrap().dest_id, Some(200));

        // Out of bounds index
        assert!(table.get_node_by_idx(2).is_none());
    }

    #[test]
    fn test_set_node_error() {
        let mut table = BmNetworkRoutingTable::new(Some(1));
        let target_id = Some(50);

        // Add node route first
        table.update_node_route(target_id, Some(50), 0, 1000, -70);

        // Set error timestamp on existing node (should not panic or fail)
        table.set_node_error(target_id, 1500);

        // Setting error on non-existent node (should log defmt error and handle gracefully)
        table.set_node_error(Some(999), 2000);
    }
}
