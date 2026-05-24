#![cfg(feature = "tonic")]
#[test]
fn envelope_roundtrips_via_prost() {
    use prost::Message;
    let env = aerro::wire::raw::Envelope {
        category: aerro::wire::raw::Category::Business as i32,
        type_id: "create_user.email_taken".into(),
        trace_id: vec![0xAA; 16].into(),
        span_id: vec![0xBB; 8].into(),
        frames: vec![],
        payload: vec![1, 2, 3].into(),
        version: 1,
    };
    let bytes = env.encode_to_vec();
    let back = aerro::wire::raw::Envelope::decode(&*bytes).unwrap();
    assert_eq!(back.type_id, env.type_id);
}
