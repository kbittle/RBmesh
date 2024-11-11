# Rb_Mesh_lib
This design takes concepts from the RadioHead library to form routes between nodes. Combined with a volatile RAM based routing table to store those paths. Message routes are prioritized by shortest distance and then by best signal strength.

Design Notes:
- Designed to be always on. The radio needs to constantly be in RX mode for the mesh to work.
- Designed with a focus on mobility. Routes between nodes can come and go, this implementation provides alot of network healing capabilities.

## Configuration:
[Config File](bm_network_configs.rs) - All compile time configurations are stored here. 

## OTA packat structure:
Every packets consists of the following structure.

+-------------+----------------+------------------+<br />
| Packet Type | Routing Header | Optional Payload |<br />
+-------------+----------------+------------------+<br />

Note: Packet Type + Header = 18 bytes. So you can legally run with the longest range Lora settings. Lowest LoRaWAN datarate settings only allow 13 bytes.

### Packet Types:
List of packet types:
```rust
pub enum BmPacketTypes {
    #[default]
    BcastNeighborTable = 0,

    RouteDiscoveryRequest = 10,
    RouteDiscoveryResponse = 11,
    RouteDiscoveryError = 12,

    DataPayload = 20,
    DataPayloadAck = 21,
}
```

### Routing Header:
All ID's are 32bit values.
+-----------+-------------+---------------+----------------+-----------+<br />
| Source ID | Next Hop ID | Originator ID | Destination ID | Info Bits |<br />
+-----------+-------------+---------------+----------------+-----------+<br />
