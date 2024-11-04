use defmt_rtt as _; // global logger
use defmt::unwrap;
use heapless::{Vec, String}; // fixed capacity `std::Vec`
use crate::bm_network::{
    NetworkId, RssiType, TimeType,
    bm_network_configs::*,
};
use core::fmt::{self};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct BmRoute {
    // Next hop address
    next_hop: NetworkId,
    // Number of hops to end point
    distance: u8,
    // Timestamp when route was updated
    timestamp_millis: TimeType,
    // Rssi for route
    avg_rssi: i32,
    rssi_samples: Vec<RssiType, BM_MAX_RSSI_SAMPLES>,
    // Failure count
    failures: u8,
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

    pub fn get_next_hop(&mut self) -> NetworkId {
        self.next_hop
    }
}

#[derive(Default, Debug, Clone)]
pub struct BmNodeEntry {
    // Node network address
    pub dest_id: NetworkId,
    // Primary route index
    primary_route_idx: Option<usize>,
    // Available routes
    routes: Vec<BmRoute, BM_MAX_DEVICE_ROUTES>,
}

impl fmt::Display for BmNodeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id: {}, Routes: {}", self.dest_id.unwrap(), self.routes.len())
    }
}

impl BmNodeEntry {
    pub fn new(dest_id: NetworkId) -> BmNodeEntry {
        BmNodeEntry {
            dest_id: dest_id,
            primary_route_idx: None,
            routes: Vec::new(),
        }
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

    pub fn record_error(&mut self, millis: TimeType) {
        if let Some(route_idx) = self.primary_route_idx {
            self.routes[route_idx].failures += 1;
            self.routes[route_idx].timestamp_millis = millis;
        }
    }

    pub fn add_new_route(&mut self, next_hop: NetworkId, distance: u8, millis: TimeType, rssi: RssiType) {
        if self.routes.len() < BM_MAX_DEVICE_ROUTES {
            defmt::info!("BmNodeEntry: add_new_route");

            // Create new route
            let mut new_route = BmRoute {
                next_hop: next_hop,
                distance: distance,
                timestamp_millis: millis,
                avg_rssi: 0,
                rssi_samples: Vec::new(),
                failures: 0,
            };
            new_route.update_rssi(rssi);

            // Add new route to list
            self.routes.push(new_route).unwrap();

            self.determine_primary_route();
        }
        else {
            defmt::error!("BmNodeEntry: route list full");
            // TODO - clean up old routes
        }        
    }

    pub fn update_route(&mut self, next_hop: NetworkId, distance: u8, millis: TimeType, rssi: RssiType) {
        let mut route_found = false;

        for route in self.routes.iter_mut() {
            // If the route exists update the route data
            if route.next_hop == next_hop {
                route_found = true;

                route.distance = distance;
                route.timestamp_millis = millis;
                route.update_rssi(rssi);
            }
        }

        // If we didnt find a route, add new route
        if !route_found {
            self.add_new_route(next_hop, distance, millis, rssi);
        }

        self.determine_primary_route();
    }

    pub fn get_best_route(&mut self) -> Option<BmRoute> {
        if let Some(route_idx) = self.primary_route_idx {
            return Some(self.routes[route_idx].clone())
        }
        None
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

    fn route_exists(&mut self, next_hop: NetworkId) -> bool {
        self.routes.iter().any(|rt| {
            rt.next_hop == next_hop
        })
    }

    fn determine_primary_route(&mut self) {
        for (index, route) in self.routes.iter().enumerate() {
            if let Some(primary_index) = self.primary_route_idx {
                // Ensure we dont compare primary rpoute against itself
                if index != primary_index {
                    if compare_routes(&self.routes[primary_index], route) {
                        // Update primary route
                        self.primary_route_idx = Some(index);
                    }
                }
            }
            else {
                // Start with first route as primary
                self.primary_route_idx = Some(index);
            }
        }
    }
}

// Function to compare two routes.
// If route2 is better than route1, return true
// Otherwise return false
fn compare_routes(route1: &BmRoute, route2: &BmRoute) -> bool {
    calc_route_metric(route1) > calc_route_metric(route2)
}

// Calculate metric based off route data. Lower metric number is better.
//
//     Hop, Rssi, Errors, Metric
// Ex. 0    -90   0       90
//     1    -87   0       117
//     2    -80   0       140
//     1    -112  1       192
fn calc_route_metric(route: &BmRoute) -> i32 {
    let mut metric: i32 = 0;

    // Prioritize closer routes
    metric += route.distance as i32 * 30;

    metric += route.avg_rssi * -1;

    // Route failures will penalize the link
    metric += route.failures as i32 * 50;

    metric
}