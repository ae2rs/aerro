//! Criterion bench: aerro encode/decode.
//!
//! Run with: cargo bench -p aerro --bench error_path

#[cfg(not(feature = "macro"))]
fn main() {
    eprintln!("error_path bench requires the `macro` feature.");
}

#[cfg(feature = "macro")]
use aerro::wire::encode::EncodeOptions;
#[cfg(feature = "macro")]
use aerro::{IntoStatus, StatusIntoResultExt};
#[cfg(feature = "macro")]
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "macro")]
#[derive(Debug, aerro::Aerro)]
pub enum Bench {
    #[aerro(category = Business, code = AlreadyExists, error = "x={x} y={y}")]
    Item { x: u64, y: String },
}

#[cfg(feature = "macro")]
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

    group.finish();
}

#[cfg(feature = "macro")]
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

#[cfg(feature = "macro")]
criterion_group!(benches, bench_encode, bench_decode);
#[cfg(feature = "macro")]
criterion_main!(benches);
