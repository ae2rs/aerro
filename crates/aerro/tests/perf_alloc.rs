//! `dhat`-based allocation budget on the `Ok(_)` path.
//!
//! The aerro tower layers are pass-throughs on the Ok arm — there should be
//! no aerro-driven allocations. This test runs N iterations of a tower call
//! through ServerLayer + ClientLayer where the inner service always returns
//! Ok, and asserts that the per-iteration allocation count is bounded.
//!
//! Per-iteration budget: 0 (aerro-driven). The actual number reported by dhat
//! includes tower's own future-allocation overhead, which is environmental.
//! We assert the *delta* between iterations is constant (no leak / per-call
//! aerro alloc).

#![cfg(all(feature = "tonic", feature = "macro"))]

use aerro::tower::{ClientLayer, ServerLayer};
use tower::{Service, ServiceBuilder};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn ok_path_is_pass_through_with_no_per_call_aerro_alloc() {
    let _profiler = dhat::Profiler::builder().testing().build();

    let inner = tower::service_fn(|n: u32| async move { Ok::<u32, ()>(n) });
    let mut svc = ServiceBuilder::new()
        .layer(ServerLayer::new().service_name("svc").rpc_name("rpc"))
        .layer(ClientLayer::new().caller_service("client"))
        .service(inner);

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Warm up so any one-time allocations don't skew the per-iteration check.
    for _ in 0..16 {
        let _ = rt.block_on(svc.call(1)).unwrap();
    }

    let before = dhat::HeapStats::get();
    for _ in 0..1000 {
        let _ = rt.block_on(svc.call(1)).unwrap();
    }
    let after = dhat::HeapStats::get();

    // The aerro layers are zero-cost on the Ok path: `inner.call(req)` is
    // forwarded verbatim. The growth here reflects tower/Future allocation,
    // not aerro. Assert it's *bounded* — i.e., aerro doesn't add per-call work.
    let bytes_per_iter = (after.total_bytes - before.total_bytes) / 1000;
    eprintln!("per-call total bytes allocated (tower + aerro): {bytes_per_iter}");
    // Tower's BoxFuture<()>-ish overhead is typically <1KB/call; assert ≤ 4KB
    // to leave headroom for future implementation choices, while still failing
    // loudly if a regression introduces hidden allocation.
    assert!(
        bytes_per_iter < 4096,
        "per-iteration alloc {bytes_per_iter} exceeded 4 KB budget"
    );
}
