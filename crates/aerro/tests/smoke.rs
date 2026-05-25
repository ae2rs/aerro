#[test]
fn workspace_smoke() {
    assert_eq!(aerro::VERSION, env!("CARGO_PKG_VERSION"));
}
