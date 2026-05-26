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
use aerro::{Aerro, Category, Frame, ServiceFailure};
use tonic::Code;

// ── Service error types ──────────────────────────────────────────────────────

/// Innermost service — plain typed errors, no Forward.
#[derive(Debug, Aerro)]
pub enum PipelineError {
    #[aerro(code = System::Internal, error = "backend.unreachable")]
    Unreachable,
}

/// Middle service — plain typed errors, no Forward (so GatewayError can decode them).
#[derive(Debug, Aerro)]
pub enum RelayError {
    #[aerro(code = System::Internal)]
    PipelineErrorFailed,
}

/// Outermost aggregating service — uses Forward to collect upstream errors.
/// Forward variants decode as RemoteError on the client side, but the full
/// frame chain is preserved.
#[derive(Debug, Aerro)]
pub enum GatewayError {
    #[aerro(code = System::Internal)]
    RelayErrorFailed(#[aerro(forward)] RelayError),
}

// ── Simulated service call chain ─────────────────────────────────────────────

fn backend() -> Result<(), PipelineError> {
    Err(PipelineError::Unreachable)
}

#[allow(clippy::result_large_err)]
fn relay() -> Result<(), tonic::Status> {
    match backend() {
        Ok(v) => Ok(v),
        Err(_) => {
            let mut sf = ServiceFailure::new(RelayError::PipelineErrorFailed);
            sf.push_frame(Frame::local(
                "backend",
                "ping",
                Code::Internal,
                "backend.unreachable",
                Category::System,
            ));
            Err(sf.encode(&EncodeOptions::default()))
        }
    }
}

/// GatewayError decodes RelayError errors typed (RelayError has no Forward variants),
/// then uses `.forward()` — the upstream frames transfer automatically.
#[allow(clippy::result_large_err)]
fn gateway() -> Result<(), tonic::Status> {
    let sf: ServiceFailure<RelayError> = relay()
        .map_err(|st| ServiceFailure::<RelayError>::try_from(st).expect("typed"))
        .unwrap_err();

    // .forward() transfers the RelayError frames into the GatewayError ServiceFailure.
    let mut sf: ServiceFailure<GatewayError> = sf.forward();

    // Optionally annotate the forwarding hop itself.
    sf.push_frame(Frame::local(
        "relay",
        "forward",
        Code::Internal,
        "relay.pipeline_failed",
        Category::System,
    ));
    Err(sf.encode(&EncodeOptions::default()))
}

// ── Client ───────────────────────────────────────────────────────────────────

fn main() {
    let st = gateway().unwrap_err();

    // GatewayError.RelayErrorFailed is a Forward variant — opaque on decode.
    // The client receives a RemoteError with the full frame chain.
    let re = ServiceFailure::<GatewayError>::try_from(st)
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
