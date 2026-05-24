//! Criterion bench: aerro encode/decode vs. a serde_json baseline.
//!
//! Run with: cargo bench -p aerro --bench error_path --features compat-json

use aerro::{IntoStatus, StatusIntoResultExt};
use aerro::wire::encode::EncodeOptions;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[aerro::operation]
pub enum Bench {
    #[aerro(category = "business", code = "already_exists", error = "x={x} y={y}")]
    Item { x: u64, y: String },
}

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");
    let opts = EncodeOptions::default();

    group.bench_function("aerro_prost", |b| {
        b.iter(|| {
            let v = Bench::Item {
                x: 42,
                y: "hello-world".into(),
            };
            black_box(v.into_status(black_box(&opts)));
        });
    });

    #[cfg(feature = "compat-json")]
    group.bench_function("compat_json", |b| {
        use aerro::ServiceFailure;
        use aerro::compat_json::encode_json;
        b.iter(|| {
            let sf: ServiceFailure<Bench> = Bench::Item {
                x: 42,
                y: "hello-world".into(),
            }
            .into();
            black_box(encode_json(&sf, black_box(&opts)));
        });
    });

    group.finish();
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");
    let opts = EncodeOptions::default();
    let prost_status = Bench::Item {
        x: 42,
        y: "hello-world".into(),
    }
    .into_status(&opts);

    group.bench_function("aerro_prost", |b| {
        b.iter(|| {
            // Clone the inner Status so each iteration sees a fresh one.
            let st = tonic::Status::with_details(
                prost_status.code(),
                prost_status.message(),
                bytes::Bytes::copy_from_slice(prost_status.details()),
            );
            black_box(st.into_aerro::<Bench>().unwrap());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_encode, bench_decode);
criterion_main!(benches);
