use super::bm_network_engine::BmNetworkEngine;

#[test]
fn my_test() {
    let mut bm_engine = BmNetworkEngine::new(Some(5));

    assert_eq!(bm_engine.get_next_outbound_packet(), None);
}
