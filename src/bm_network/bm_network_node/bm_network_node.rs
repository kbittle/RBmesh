use defmt_rtt as _; // global logger
use defmt::unwrap;
use heapless::{Vec, String}; // fixed capacity `std::Vec`
use crate::bm_network::{
    NetworkId, RssiType, TimeType,
    bm_network_configs::*,
};
use core::fmt::{self};


#[derive(Default, Debug, Clone)]
struct BmNodeMetrics {
    // Time when node was last heard from
    timestamp_millis: TimeType,
    
    errors: u8,
}

#[derive(Default, Debug, Clone, PartialEq)]
struct BmRoute {
    // Next hop address
    next_hop: NetworkId,
    // Number of hops to end point
    distance: u8,
    // Timestamp when route was updated
    timestamp_millis: TimeType,
    // Rssi for route
    avg_rssi: i32,
    rssi_samples: Vec<RssiType, BM_MAX_RSSI_SAMPLES>,
}

impl BmRoute {
    fn update_rssi(&mut self, _rssi:RssiType) {
        // Remove oldest sample if full
        if self.rssi_samples.is_full() {
            self.rssi_samples.pop();
        }
        // Add newest sample
        self.rssi_samples.push(_rssi).unwrap();

        // Avergae rssi samples
        self.avg_rssi = self.rssi_samples.iter().map(|&rssi| rssi as i32).sum::<i32>() / self.rssi_samples.len() as i32;
    }
}

#[derive(Default, Debug, Clone)]
pub struct BmNodeEntry {
    // Node network address
    pub dest_id: NetworkId,
    // Metrics for node
    node_metrics: BmNodeMetrics,
    // Primary route index
    primary_route_idx: i8,
    // Available routes
    routes: Vec<BmRoute, BM_MAX_DEVICE_ROUTES>,
}

impl fmt::Display for BmNodeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id: {}, Routes: {})", self.dest_id.unwrap(), self.routes.len())
    }
}

impl BmNodeEntry {
    pub fn new(dest_id: NetworkId) -> BmNodeEntry {
        BmNodeEntry {
            dest_id: dest_id,
            node_metrics: BmNodeMetrics::default(),
            primary_route_idx: 0,
            routes: Vec::new(),
        }
    }

    pub const fn with_metrics(mut self, millis: TimeType) -> Self {
        self.node_metrics.timestamp_millis = millis;
        self.node_metrics.errors = 0;
        self
    }

    pub fn with_route(mut self, next_hop: NetworkId, distance: u8, millis: TimeType, rssi: RssiType) -> Self {
        if self.route_exists(next_hop) {
            self.update_route(next_hop, distance, millis, rssi);
        }
        else {
            self.add_new_route(next_hop, distance, millis, rssi);       
        }
        self
    }

    pub fn update_metrics(&mut self, src_id: NetworkId, millis: TimeType, rssi: RssiType) {
        // not sure what to do with these yet
    }

    pub fn add_new_route(&mut self, next_hop: NetworkId, distance: u8, millis: TimeType, rssi: RssiType) {
        if self.routes.len() < BM_MAX_DEVICE_ROUTES {
            // Create new route
            let mut new_route = BmRoute {
                next_hop: next_hop,
                distance: distance,
                timestamp_millis: millis,
                avg_rssi: 0,
                rssi_samples: Vec::new(),
            };
            new_route.update_rssi(rssi);
            // Add new route to list
            self.routes.push(new_route).unwrap();
        }
        else {
            defmt::error!("BmNodeEntry: route list full");
            // todo - clean up old routes
        }        
    }

    pub fn update_route(&mut self, next_hop: NetworkId, distance: u8, millis: TimeType, rssi: RssiType) {
        for route in self.routes.iter_mut() {
            // If the route exists update the route data
            if route.next_hop == next_hop {
                route.distance = distance;
                route.timestamp_millis = millis;
                route.update_rssi(rssi);
            }
        }

        // TODO - if we didnt find a route, add new route?, clean up stale routes
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

    fn route_exists(&mut self, next_hop: NetworkId) -> bool {
        self.routes.iter().any(|rt| {
            rt.next_hop == next_hop
        })
    }

}