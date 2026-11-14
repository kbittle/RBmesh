
// Max number of network id's the device can remember. 
pub const BM_MAX_NET_DEVICES: usize = 100;

// Number of retries allowed on acknowledged data packets.
pub const BM_PACKET_RETRY_COUNT: u8 = 2;

// Number of retries allowed on network discovery requests.
pub const BM_DISCOVERY_RETRY_COUNT: u8 = 1;

// Timeout for a network discovery request in milliseconds.
pub const BM_DISCOVERY_TIMEOUT_MILLIS: i64 = 10000;

// Timeout for an acknowledged data packet in milliseconds.
pub const BM_PACKET_ACK_TIMEOUT_MILLIS: i64 = 10000;

// Pkt type + Sizeof(BmNetworkPacketHdr)
pub const BM_PACKET_HDR_SIZE: usize = 18;

// Max number of bytes paylaod can support. This should be 255 - sizeof(hdr).
pub const BM_MAX_PAYLOAD_SIZE: usize = 200;

// Max OTA size = hdr + payload
pub const BM_MAX_OTA_SIZE: usize = BM_PACKET_HDR_SIZE + BM_MAX_PAYLOAD_SIZE;

// Max routes stored per device
pub const BM_MAX_DEVICE_ROUTES: usize = 5;

// Max rssi samples stored per route
pub const BM_MAX_RSSI_SAMPLES: usize = 5;

// Outbound queue size. Up this later? What if we hold packets for retries?
pub const BM_OUTBOUND_QUEUE_SIZE: usize = 5;

// Inbound queue size. 
pub const BM_INBOUND_QUEUE_SIZE: usize = 5;

// NOTE: stack currently lives in ram, so it cannot be that large at the moment.
// maybe can move some parts to flash some day?
//
// Chip has: 256-Kbyte Flash memory, 64-Kbyte RAM
// Randomized TX delay range (ms) used to stagger relayed packets.
pub const BM_TX_RANDOM_DELAY_MIN_MILLIS: i64 = 10;
pub const BM_TX_RANDOM_DELAY_MAX_MILLIS: i64 = 100;
pub const BM_TX_DELAY_MIN_MILLIS: i64 = BM_TX_RANDOM_DELAY_MIN_MILLIS;
pub const BM_TX_DELAY_MAX_MILLIS: i64 = BM_TX_RANDOM_DELAY_MAX_MILLIS;
pub const BM_RELAY_TX_DELAY_MIN_MILLIS: i64 = BM_TX_RANDOM_DELAY_MIN_MILLIS;
pub const BM_RELAY_TX_DELAY_MAX_MILLIS: i64 = BM_TX_RANDOM_DELAY_MAX_MILLIS;
pub const BM_RANDOM_TX_DELAY_MILLIS: i64 = BM_TX_RANDOM_DELAY_MAX_MILLIS;
pub const BM_TX_RANDOM_DELAY_MILLIS: i64 = BM_TX_RANDOM_DELAY_MAX_MILLIS;

// Periodic neighbor table broadcast feature toggle.
pub const BM_NEIGHBOR_TABLE_TX_ENABLED: bool = true;
pub const BM_ENABLE_PERIODIC_NEIGHBOR_TABLE_TX: bool = BM_NEIGHBOR_TABLE_TX_ENABLED;
pub const BM_PERIODIC_NEIGHBOR_TABLE_TX_ENABLED: bool = BM_NEIGHBOR_TABLE_TX_ENABLED;
pub const BM_NEIGHBOR_TABLE_BROADCAST_ENABLED: bool = BM_NEIGHBOR_TABLE_TX_ENABLED;

// Period between periodic neighbor table transmits (ms). Default: 2 minutes.
pub const BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS: i64 = 120000;
pub const BM_NEIGHBOR_TABLE_TX_PERIOD_MS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_BROADCAST_PERIOD_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_BROADCAST_PERIOD_MS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_PERIOD_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_PERIOD_MS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_PERIODIC_NEIGHBOR_TABLE_TX_PERIOD_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_TX_INTERVAL_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;

// Maximum number of neighbors included in a neighbor table packet.
pub const BM_NEIGHBOR_TABLE_MAX_NEIGHBORS: usize = 3;
pub const BM_MAX_NEIGHBOR_TABLE_ENTRIES: usize = BM_NEIGHBOR_TABLE_MAX_NEIGHBORS;

// Additional aliases for configurability/compatibility.
pub const BM_NEIGHBOR_TABLE_TX_PERIOD: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_PERIOD: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_INTERVAL_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_INTERVAL_MS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_INTERVAL: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_NEIGHBOR_TABLE_FREQUENCY_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_PERIODIC_NEIGHBOR_TABLE_PERIOD_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_PERIODIC_NEIGHBOR_TABLE_PERIOD_MS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;
pub const BM_PERIODIC_NEIGHBOR_TABLE_INTERVAL_MILLIS: i64 = BM_NEIGHBOR_TABLE_TX_PERIOD_MILLIS;

pub const BM_NEIGHBOR_TABLE_ENABLED: bool = BM_NEIGHBOR_TABLE_TX_ENABLED;
pub const BM_ENABLE_NEIGHBOR_TABLE: bool = BM_NEIGHBOR_TABLE_TX_ENABLED;
pub const BM_PERIODIC_NEIGHBOR_TABLE_ENABLED: bool = BM_NEIGHBOR_TABLE_TX_ENABLED;
pub const BM_PERIODIC_NEIGHBOR_TABLE_TX: bool = BM_NEIGHBOR_TABLE_TX_ENABLED;
