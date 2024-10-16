use heapless::Vec; // fixed capacity `std::Vec`

const BM_MAX_DEVICE_ROUTES: usize = 5;

type NetworkId = u32;

#[derive(Default, Debug, Clone, PartialEq)]
struct BmNodeMetrics {
    avg_rssi: i8,
    rssi_samples: Vec<u8, 10>,
    last_seen_timestamp_s: u32,
    errors: u8,
}

impl BmNodeMetrics {
    fn update_rssi(&mut self, _rssi:u8) {
        // todo - avergae rssi samples
        self.avg_rssi = _rssi;
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
struct BmRoute {
    // Next hop address
    nextHop: NetworkId,
    // Number of hops to end point
    distance: i8,
    // Timestamp when route was updated
    timestamp_sec: u32,
}

#[derive(Default, Debug, Clone, PartialEq)]
struct BmNodeEntry {
    // Node network address
    remote_net_id: NetworkId,
    // Primary route index
    primary_route_idx: i8,
    // Available routes
    routes: Vec<BmRoute: BM_MAX_DEVICE_ROUTES>,
    // Metrics for node
    node_metrics: BmNodeMetrics,
}

impl BmNodeEntry {
    fn set_net_id(&mut self, _network_id:NetworkId) {
        self.remote_net_id = _network_id;
    }

    fn add_new_route(&mut self) {

    }

    fn update_rssi(&mut self, _rssi: u8) {
        self.node_metrics.update_rssi(_rssi);
    }
}