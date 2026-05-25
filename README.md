# aerro

[![Crates.io](https://img.shields.io/crates/v/aerro.svg)](https://crates.io/crates/aerro)
[![docs.rs](https://img.shields.io/docsrs/aerro)](https://docs.rs/aerro)
[![CI](https://github.com/ae2rs/aerro/actions/workflows/ci.yml/badge.svg)](https://github.com/ae2rs/aerro/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/aerro.svg)](LICENSE-MIT)

Cross-service gRPC error framework for Rust.

`aerro` gives every error a **typed identity**, a **bounded call trace**, and a
**structured wire encoding** — so the client that receives a `tonic::Status` can
recover the original variant, read the chain of service hops it passed through,
and decide whether to surface the message to the end user.

## Features

- **Typed errors** — derive `Aerro` on any enum; each variant carries a category,
  gRPC status code, and structured message template
- **Bounded call traces** — every hop appends a `Frame`; frames are elided to a
  configurable cap on the wire so large fan-outs stay bounded
- **Exposure control** — `Internal`, `Trusted`, and `Public` tiers redact system
  errors and strip call traces automatically at the egress point
- **Zero allocations on the happy path** — no heap work when there is no error
- **Compat bridges** — optional `anyhow`, `eyre`, and JSON-envelope features

## Quick Start

Add `aerro` to your `Cargo.toml`:

```toml
[dependencies]
aerro = "0.2"
```

Define your errors, encode them into a `tonic::Status`, and recover them on the
other side:

```rust
use aerro::{Aerro, IntoStatus, StatusIntoResultExt};
use aerro::wire::encode::EncodeOptions;

#[derive(Debug, aerro::Aerro)]
pub enum CreateUserError {
    #[aerro(category = Business, code = AlreadyExists, error = "email already taken: {email}")]
    EmailTaken { email: String },

    #[aerro(category = System, code = Internal, error = "db.unavailable")]
    DbUnavailable,
}

// Server side — convert a typed failure to a tonic::Status.
let err = CreateUserError::EmailTaken { email: "alice@example.com".into() };
let status = err.into_status(&EncodeOptions::default());

// Client side — recover the original typed variant.
let recovered = status.into_aerro::<CreateUserError>().unwrap();
```

## Examples

| Example | What it shows |
|---------|---------------|
| [`basic`](crates/aerro/examples/basic.rs) | Minimum viable usage — one enum, one wire round-trip |
| [`handler`](crates/aerro/examples/handler.rs) | `#[derive(AerroHandler)]` for typed RPC handlers |
| [`trace_chain`](crates/aerro/examples/trace_chain.rs) | 3-hop trace accumulation across service boundaries |
| [`exposure`](crates/aerro/examples/exposure.rs) | `Internal` / `Trusted` / `Public` redaction tiers |
| [`compat`](crates/aerro/examples/compat.rs) | JSON envelope alternative (`compat-json` feature) |

Run any example with:

```
cargo run --example basic --features macro
```

## Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `macro` | ✓ | `#[derive(Aerro)]` and `#[derive(AerroHandler)]` proc-macros |
| `tracing` | ✓ | Capture OTel trace/span IDs from the active `tracing` span |
| `anyhow` | — | `AnyError` bridge for `anyhow::Error` |
| `eyre` | — | `AnyError` bridge for `eyre::Report` |
| `compat-json` | — | JSON wire envelope alternative to the default protobuf encoding |

## Status

**Alpha.** The core wire format and derive macros are functional and the API is
stabilising, but has not yet been used in production. Expect minor breaking
changes in the 0.x series.

Feedback, issues, and PRs are welcome.

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
