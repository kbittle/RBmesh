
// Max number of network id's the device can remember. 
pub const BM_MAX_NET_DEVICES: usize = 100;

// Number of retries allowed on acknowledged data packets.
pub const BM_PACKET_RETRY_COUNT: u8 = 2;

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

// NOTE: stack currently lives in ram, so it cannot be that large at the moment.
// maybe can move some parts to flash some day?
//
// Chip has: 256-Kbyte Flash memory, 64-Kbyte RAM