use defmt::unwrap;
use defmt_rtt as _; // global logger
use heapless::Vec; // fixed capacity `std::Vec`
use crate::bm_network_stack::{NetworkId, RssiType};

const BM_MAX_DEVICE_ROUTES: usize = 5;

#[derive(Default, Debug, Clone)]
struct BmNodeMetrics {
    avg_rssi: i32,
    rssi_samples: Vec<RssiType, 10>,
    last_seen_timestamp_s: u32,
    errors: u8,
}

impl BmNodeMetrics {
    fn update_rssi(&mut self, _rssi:RssiType) {
        // Remove oldest sample if full
        if self.rssi_samples.is_full() {
            self.rssi_samples.pop();
        }
        // Add newest sample
        self.rssi_samples.push(_rssi).unwrap();

        // Avergae rssi samples
        self.avg_rssi = self.rssi_samples.iter().map(|&rssi| rssi as i32).sum::<i32>() / self.rssi_samples.len() as i32;

        // Todo - figure out timestamp stuff
        self.last_seen_timestamp_s = 0;

        self.errors = 0;
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
struct BmRoute {
    // Next hop address
    next_hop: NetworkId,
    // Number of hops to end point
    distance: i8,
    // Timestamp when route was updated
    timestamp_sec: u32,
}

#[derive(Default, Debug, Clone)]
pub struct BmNodeEntry {
    // Node network address
    pub dest: NetworkId,
    // Primary route index
    primary_route_idx: i8,
    // Available routes
    routes: Vec<BmRoute, BM_MAX_DEVICE_ROUTES>,
    // Metrics for node
    node_metrics: BmNodeMetrics,
}

impl BmNodeEntry {
    pub fn new() -> BmNodeEntry {
        BmNodeEntry {
            dest: None,
            primary_route_idx: 0,
            routes: Vec::new(),
            node_metrics: BmNodeMetrics::default(),
        }
    }

    pub fn add_new_route(&mut self, new_route: BmRoute) -> Result<(), &str> {
        if self.routes.len() < BM_MAX_DEVICE_ROUTES {
            self.routes.push(new_route).unwrap()

            // todo - evaluate routes
        }
        // Todo how to clean up old routes??
        return Err("Too many routes");        
    }

    pub fn update_rssi(&mut self, _rssi: RssiType) {
        self.node_metrics.update_rssi(_rssi);
    }
}