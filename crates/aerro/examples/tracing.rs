//! Shows that aerro automatically captures the active OTel span's trace ID
//! and span ID when an error is created. The IDs printed by aerro match
//! those shown in the exported span — enabling error-to-trace correlation.
//!
//! Run with: cargo run --example tracing --features macro,tracing

use aerro::{Aerro, AerroEncode, ServiceFailure};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Aerro)]
pub enum ApiError {
    #[aerro(code = Business::NotFound)]
    UserNotFound,
}

fn init_tracing() -> SdkTracerProvider {
    let exporter = opentelemetry_stdout::SpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("aerro-tracing-example");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry().with(otel_layer).init();
    provider
}

fn main() {
    let provider = init_tracing();

    // Scope the span so it is dropped (and exported) before we print or shutdown.
    let (trace_id, span_id) = {
        let span = tracing::info_span!("handle_get_user");
        let status = {
            let _enter = span.enter();
            ApiError::UserNotFound.encode()
        };
        let sf = ServiceFailure::<ApiError>::try_from(status).unwrap();
        let t = sf.trace();
        (t.trace_id, t.span_id)
        // span dropped here → SimpleSpanProcessor exports it synchronously
    };

    // Flush remaining spans (none here, but good practice).
    let _ = provider.shutdown();

    // Print after shutdown so the exported span JSON appears first.
    println!();
    println!("aerro trace_id = {:032x}", u128::from_be_bytes(trace_id));
    println!("aerro span_id  = {:016x}", u64::from_be_bytes(span_id));
    println!();
    println!("The trace_id and span_id above should match the span exported above.");
}
