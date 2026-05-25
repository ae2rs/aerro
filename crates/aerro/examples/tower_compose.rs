//! Show ServerLayer composing with other tower layers via ServiceBuilder.

use aerro::Exposure;
use aerro::tower::ServerLayer;
use tower::{Service, ServiceBuilder};

fn main() {
    let inner = tower::service_fn(|n: u32| async move { Ok::<u32, ()>(n * 2) });
    let mut svc = ServiceBuilder::new()
        .layer(
            ServerLayer::new()
                .service_name("doubler")
                .rpc_name("call")
                .exposure(Exposure::Trusted)
                .max_frames(8),
        )
        .service(inner);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let out = rt.block_on(svc.call(21)).unwrap();
    println!("doubler(21) = {out}");
    println!(
        "layer carries service_name=create-user equivalent? (held in layer for the handler macro to read)"
    );
}
