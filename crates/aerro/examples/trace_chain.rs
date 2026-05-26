//! 3-hop trace accumulation: backend (B) → middleware (M) → gateway (G).
//!
//! Demonstrates how `#[aerro(forward)]` + `.forward()` makes the gateway hop
//! ergonomic: no manual frame push needed to transfer the upstream chain.
//!
//! Constraint: Forward variants are opaque on decode. Therefore only the
//! *innermost* services (B, M) should use typed plain errors; the *outermost*
//! aggregating service (G) uses Forward to collect them. The final client
//! receives a `RemoteError` with the full accumulated frame list.

use aerro::wire::encode::EncodeOptions;
use aerro::{Aerro, Category, Frame, IntoStatus, ServiceFailure, StatusIntoResultExt};
use tonic::Code;

// ── Service error types ──────────────────────────────────────────────────────

/// Innermost service — plain typed errors, no Forward.
#[derive(Debug, Aerro)]
pub enum Pipeline {
    #[aerro(code = System::Internal, error = "backend.unreachable")]
    Unreachable,
}

/// Middle service — plain typed errors, no Forward (so Gateway can decode them).
#[derive(Debug, Aerro)]
pub enum Relay {
    #[aerro(code = System::Internal)]
    PipelineFailed,
}

/// Outermost aggregating service — uses Forward to collect upstream errors.
/// Forward variants decode as RemoteError on the client side, but the full
/// frame chain is preserved.
#[derive(Debug, Aerro)]
pub enum Gateway {
    #[aerro(code = System::Internal)]
    RelayFailed(#[aerro(forward)] Relay),
}

// ── Simulated service call chain ─────────────────────────────────────────────

fn backend() -> Result<(), Pipeline> {
    Err(Pipeline::Unreachable)
}

#[allow(clippy::result_large_err)]
fn relay() -> Result<(), tonic::Status> {
    match backend() {
        Ok(v) => Ok(v),
        Err(_) => {
            let mut sf = ServiceFailure::new(Relay::PipelineFailed);
            sf.push_frame(Frame::local(
                "backend",
                "ping",
                Code::Internal,
                "backend.unreachable",
                Category::System,
            ));
            Err(sf.into_status(&EncodeOptions::default()))
        }
    }
}

/// Gateway decodes Relay errors typed (Relay has no Forward variants),
/// then uses `.forward()` — the upstream frames transfer automatically.
#[allow(clippy::result_large_err)]
fn gateway() -> Result<(), tonic::Status> {
    let sf: ServiceFailure<Relay> = relay()
        .map_err(|st| st.into_aerro::<Relay>().expect("typed"))
        .unwrap_err();

    // .forward() transfers the Relay frames into the Gateway ServiceFailure.
    let mut sf: ServiceFailure<Gateway> = sf.forward();

    // Optionally annotate the forwarding hop itself.
    sf.push_frame(Frame::local(
        "relay",
        "forward",
        Code::Internal,
        "relay.pipeline_failed",
        Category::System,
    ));
    Err(sf.into_status(&EncodeOptions::default()))
}

// ── Client ───────────────────────────────────────────────────────────────────

fn main() {
    let st = gateway().unwrap_err();

    // Gateway.RelayFailed is a Forward variant — opaque on decode.
    // The client receives a RemoteError with the full frame chain.
    let re = st
        .into_aerro::<Gateway>()
        .expect_err("Forward variants decode as RemoteError");

    println!("frames on final hop:");
    for (i, f) in re.frames().iter().enumerate() {
        println!(
            "  {i}: service={} rpc={} code={:?} msg={}",
            f.service, f.rpc, f.code, f.message
        );
    }
    println!("trace_id={:02x?}", re.trace().trace_id);
}
