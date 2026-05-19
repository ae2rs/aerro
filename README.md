# aerro

`aerro` is an early-stage Rust library for cross-service gRPC errors.

The goal is to provide a complete error framework for service-oriented Rust
systems using `tonic`, `prost`, and the broader gRPC ecosystem:

- full call traces across service boundaries
- typed error upcasting and downcasting
- efficient transport through gRPC status metadata and protobuf details
- low overhead abstractions suitable for high-throughput services
- compatibility with established Rust error tooling where it is a good fit

This first release reserves the crate name while the public API is designed.

## Status

The crate is not ready for production use yet.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
