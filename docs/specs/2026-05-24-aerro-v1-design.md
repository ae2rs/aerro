# aerro — Design Spec (v1)

> **Status.** Approved design synthesised from the 2026-05-24 brainstorm. No implementation work in this document — it is the foundation for the subsequent implementation plan(s). The current `src/lib.rs` is a 387-byte stub; treat this document as the source of truth, not the memory of older prototypes.

---

## 1. Context

`aerro` is the open-source Rust gRPC error library that this crate is built around. Today the repo is a name reservation plus README — `src/lib.rs` exposes a `VERSION` constant and nothing else; there are no dependencies, no proc-macro crate on disk, and no examples. Earlier notes describing `typed/`, `layer/`, `wire/`, `Detail`, and `RemoteError` modules refer to prototypes that no longer exist.

Two reference implementations inform the design:

- **`/Users/lucas/perso/uni`** ships `#[service_error]`: per-RPC error enums with stable string IDs, a `Business` / `System` split, JSON in `Status.details()`, and a fallback `Internal(AnyError)` variant. There is **no cross-service trace** — errors collapse to a string once they cross a hop.
- **`/Users/lucas/work/monorepo`** uses the same pattern at production scale: ~846 `service_error` enums, ~400 protos, hundreds of daemons. The visible pain points are JSON serialization cost on hot paths, opacity to unknown clients, hard-coded retry/availability detection by chain-walking, and no structured retry metadata.

aerro inherits uni's per-RPC enum discipline (it's the right model) but answers what those projects don't: **bounded cross-service call traces inside the error**, **structured upcasting/downcasting against unknown variants**, **zero allocations on the happy path**, and **explicit exposure policy** so a single error type can flow across both internal and public surfaces. The library is gRPC-first today but layered to extend to other transports later.

---

## 2. Philosophy

1. **Per-RPC typed errors are non-negotiable.** A gRPC method has a typed request, a typed response, and therefore a typed error. The macro enforces this, the same way uni does.
2. **A bounded inline trace beats correlation IDs alone.** The error is self-describing enough to debug without the observability backend, but bounded enough not to blow up payload size under fan-out. OTel IDs are always present so the full chain can still be reconstructed.
3. **Coexist with anyhow/eyre, don't replace them.** Idiomatic `?`-chained service code stays the same. Conversion to the typed enum happens at the wire boundary.
4. **Zero cost on `Ok(_)`, full structure on `Err(_)`.** The error path is allowed to allocate, encode, and capture; the success path must do none of it.
5. **Two integration modes, one semantics.** Tower layers for composability with the rest of the tower ecosystem; an inline macro adapter for the hottest paths and build-time validation. Same envelope, same trace model.
6. **Polyglot-safe by default.** A Go or Python client that doesn't know aerro must still see correct `Status.code()` and `Status.message()`. The structured payload is additive, never required.

---

## 3. Goals and Non-goals

**Goals (v1):**

- Typed error round-trip across one or more tonic hops with zero loss of structure for known types.
- Bounded inline frame stack + OTel trace IDs on every error.
- Strongly-typed downcast (`TryFromStatus<E>`) with a structured `RemoteError` fallback when the wire type is unknown.
- Per-variant `Category` (Business / System / Validation / Transport) and `Exposure` (Internal / Trusted / Public) with route-level enforcement.
- Anyhow/eyre interop via `#[source]` / `#[from] AnyError`.
- Zero allocations and zero atomic ops on `Ok(_)`, asserted in CI.
- Tower layer integration **and** macro adapter integration, feature-gated, sharing the same encoder/decoder core.

**Non-goals (v1, parking lot):**

- Non-gRPC transports (the layer abstraction reserves room, but no shipped adapters).
- Multi-error aggregation / Result accumulation.
- Runtime-pluggable payload codecs (bincode is the only payload codec in v1).
- Custom domain-specific status codes beyond gRPC's 16.
- Audit logs, error history, or causal database.
- A replacement for `tracing` / OTel — aerro reads from them, doesn't reimplement them.

---

## 4. Crate layout and feature flags

Two-crate Cargo workspace at the repo root:

```
aerro/
├── Cargo.toml                # workspace manifest
├── crates/
│   ├── aerro/                # library crate (the published `aerro`)
│   └── aerro-macros/         # proc-macro crate
├── examples/
└── benches/
```

**Workspace members:** `aerro`, `aerro-macros`. This is a structural change from the current single-crate layout at the repo root (`src/lib.rs`, root `Cargo.toml`); the v1 implementation plan begins by moving the existing stub into `crates/aerro/` and adding the `aerro-macros` member.

**`aerro` features:**

| Feature | Default | Purpose |
|---|---|---|
| `tonic` | yes | tower layers + `Status` extension traits |
| `macro` | yes | re-exports `aerro-macros` |
| `tracing` | yes | reads `trace_id` / `span_id` from current span |
| `anyhow` | no | `AnyError = anyhow::Error` alias |
| `eyre` | no | `AnyError = eyre::Report` alias |
| `compat-json` | no | reads/writes uni/monorepo-style JSON details for migration |

Exactly one of `anyhow` / `eyre` may be enabled at a time; both off is allowed (no `AnyError` alias, users may still embed a custom error type as a source).

**MSRV / edition:** Rust 1.85, edition 2024 (matches the current `Cargo.toml`). MSRV is held for at least one calendar year before bumping.

**Module organisation rule.** Every `lib.rs` and `mod.rs` contains only `mod` declarations, re-exports, and crate-level docs. Implementation lives in sibling files.

---

## 5. Core types

All public types live behind the `aerro` crate root re-exports.

```rust
/// Universal trait implemented by every typed error.
pub trait Aerro: std::error::Error + Send + Sync + 'static {
    /// Stable, version-pinned identifier (e.g. "create_user.email_taken").
    fn type_id(&self) -> &'static str;

    /// One of the four taxonomy buckets.
    fn category(&self) -> Category;

    /// gRPC code this variant maps to.
    fn code(&self) -> tonic::Code;

    /// Exposure declared on this variant (after attribute overrides).
    fn exposure(&self) -> Exposure;

    /// Encode the variant's payload into bincode bytes.
    /// Implementations are generated by the derive; manual impls are allowed.
    fn encode_payload(&self, buf: &mut Vec<u8>);

    /// Decode a typed variant from a `type_id` + bincode bytes.
    fn decode_payload(type_id: &str, bytes: &[u8]) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

#[non_exhaustive]
pub enum Category { Business, System, Validation, Transport }

#[non_exhaustive]
pub enum Exposure { Internal, Trusted, Public }

pub struct Frame {
    pub service: Cow<'static, str>,
    pub rpc:     Cow<'static, str>,
    pub code:    tonic::Code,
    pub message: Cow<'static, str>,
    pub location: Option<&'static std::panic::Location<'static>>, // None for received frames
    pub category: Category,
}

pub struct ServiceFailure<E: Aerro> {
    pub inner:  E,
    pub frames: smallvec::SmallVec<[Frame; 4]>,
    pub trace:  TraceContext, // { trace_id: [u8; 16], span_id: [u8; 8] } — zeros when tracing feature off
}

/// Type-erased fallback when the wire payload's `type_id` is unknown to the caller.
pub struct RemoteError {
    pub category: Category,
    pub type_id:  String,
    pub frames:   smallvec::SmallVec<[Frame; 4]>,
    pub trace:    TraceContext,
    pub outer_code: tonic::Code,
    pub outer_message: String,
    payload_bytes: bytes::Bytes,
}

impl RemoteError {
    /// Try to recover a known concrete type.
    pub fn downcast<E: Aerro>(&self) -> Option<E> { /* ... */ }
}

pub trait IntoStatus {
    fn into_status(self, opts: &EncodeOptions) -> tonic::Status;
}

pub trait TryFromStatus<E: Aerro>: Sized {
    /// Returns `Ok(ServiceFailure<E>)` if the wire type matches `E`,
    /// `Err(RemoteError)` if it doesn't (including bare transport Statuses).
    fn try_from_status(status: tonic::Status) -> Result<ServiceFailure<E>, RemoteError>;
}

pub struct EncodeOptions {
    pub exposure:   Exposure,
    pub max_frames: u8, // default 16
}
```

`ServiceFailure<E>` is what handlers return when they want to attach context manually. The macro and layers wrap a bare `E` automatically.

---

## 6. Wire envelope

A single `prost`-encoded message in `tonic::Status::details()`. The `.proto` lives in `crates/aerro/proto/aerro.v1.proto` and is built at compile time (no codegen output checked in; build script uses `tonic-build` minus the service half).

```proto
syntax = "proto3";
package aerro.v1;

message Envelope {
  Category          category  = 1;
  string            type_id   = 2; // stable, e.g. "create_user.email_taken"
  bytes             trace_id  = 3; // 16 bytes (W3C trace-context), empty if tracing off
  bytes             span_id   = 4; // 8 bytes
  repeated Frame    frames    = 5;
  bytes             payload   = 6; // bincode of the typed variant's fields
  uint32            version   = 7; // envelope schema version, starts at 1
}

message Frame {
  string   service  = 1;
  string   rpc      = 2;
  uint32   code     = 3; // grpc::Code as u32, stable
  string   message  = 4;
  string   location = 5; // "file:line", empty for received frames
  Category category = 6;
}

enum Category {
  CATEGORY_UNSPECIFIED = 0;
  CATEGORY_BUSINESS    = 1;
  CATEGORY_SYSTEM      = 2;
  CATEGORY_VALIDATION  = 3;
  CATEGORY_TRANSPORT   = 4;
}
```

**Redundancy with `Status`.** `tonic::Status::code()` is set to the outermost frame's `code`, and `tonic::Status::message()` is set to the redacted-for-exposure human message. A polyglot client that doesn't parse `details()` still gets a useful response; aerro-aware clients get the structured envelope on top.

**`compat-json`.** When the `compat-json` feature is enabled, the encoder additionally writes a uni/monorepo-shaped JSON blob into a second `details()` slot; the decoder will fall back to JSON if the prost envelope is absent. This is a migration aid, not a v1-default behaviour.

---

## 7. Trace model

- **Server-side append.** When the server-side encoder converts `Err(E)` into a `Status`, it constructs a `Frame { service, rpc, code, message, location: Some(track_caller), category }` and pushes it onto the frame stack already present in `ServiceFailure<E>`.
- **Client-side append.** When the client-side decoder converts `Status` back into `ServiceFailure<E>` (or `RemoteError`), it appends one `Frame { service: <caller>, rpc: <method>, code: outer, message: outer, location: Some(track_caller), category: same }` on top of whatever frames the wire carried.
- **Cap and elision.** The default cap is 16 frames. When pushing would exceed it, the middle is collapsed into a synthetic frame `{ service: "...", rpc: "elided", message: "{n} frames elided", category: same-as-removed-frames }`. Cap is configurable via `EncodeOptions { max_frames }`.
- **OTel link.** When the `tracing` feature is on, `trace_id`/`span_id` are filled from the current span's `OpenTelemetrySpanExt` context (via `tracing-opentelemetry`); when off, or when no OTel layer is installed, both fields are zero-length and aerro-aware clients can fall back to log correlation.
- **Locations.** Stored as `&'static std::panic::Location<'static>` locally (zero allocation, captured via `#[track_caller]`). When serialized for the wire, formatted to `"file:line"`. Received frames cannot recover a `'static Location` and store `None` locally with the wire string in `Cow::Owned`.

---

## 8. Category semantics

| Category | gRPC codes | Default exposure | Notes |
|---|---|---|---|
| Business | InvalidArgument, NotFound, AlreadyExists, FailedPrecondition, PermissionDenied, Unauthenticated, OutOfRange, ResourceExhausted, Aborted | Public | Caller fault. Safe to surface to public callers. |
| System | Internal, Unknown, DataLoss | Internal | Server defect. Message is replaced with a fixed sentinel at non-Internal exposure. |
| Validation | InvalidArgument | Public | Carved out so a malformed proto isn't mis-classified as a server defect. Distinct retry semantics (never retry). |
| Transport | Unavailable, DeadlineExceeded, Cancelled | Trusted | Almost never user-declared. The client layer synthesises a `Transport` `RemoteError` when a bare `Status` arrives without an aerro envelope. |

`#[non_exhaustive]` on `Category` so new categories don't break downstream `match` arms.

---

## 9. Exposure and redaction

Three exposure tiers, ordered: `Internal < Trusted < Public`.

- **Variant attribute.** `#[aerro(exposure = "public" | "trusted" | "internal")]` on a variant overrides the category default for that variant only.
- **Field attribute.** `#[aerro(redact)]` on a field strips it on egress at any exposure *below* `Internal`. The bincode payload writes the field's `Default::default()` instead. Source chains (anyhow/eyre) are always stripped below `Internal`.
- **Route enforcement.** `ServerLayer::with_exposure(Exposure::Public)` (or the equivalent `#[aerro::handler(exposure = "public")]`) sets the **minimum** exposure for that route. The encoder clamps each variant down to `min(variant_exposure, route_exposure)` and applies the corresponding stripping pass:
  - At `Public`: frames dropped, source chains dropped, redacted fields zeroed, `System`-category messages replaced with `"internal error"`.
  - At `Trusted`: frames kept, redacted fields zeroed, `Internal`-exposure variants' messages replaced.
  - At `Internal`: everything ships.

An encoder never *upgrades* exposure — a variant declared `Internal` cannot leak out of a `Public` route by misconfiguration.

---

## 10. Per-RPC enum macro

```rust
use aerro::{Aerro, AnyError};

#[aerro::operation]
pub enum CreateUser {
    #[aerro(category = "business", code = "already_exists", exposure = "public")]
    EmailTaken { email: String },

    #[aerro(category = "validation", code = "invalid_argument", exposure = "public")]
    InvalidName(#[redact] String),

    #[aerro(category = "system", code = "internal")]
    Db(#[source] AnyError),

    #[aerro(category = "system", code = "internal", from)]
    Internal(#[from] AnyError),
}
```

**Generated impls:**

- `Debug` (automatic via `thiserror`).
- `thiserror::Error` derive (each variant's `Display` is the `error = "..."` format string, or a snake_case fallback).
- `Aerro` impl: `type_id` is `"<enum_snake>.<variant_snake>"`, `category` / `code` / `exposure` from attributes, payload encode/decode wired to bincode.
- `IntoStatus` for `Self`, `IntoStatus` for `ServiceFailure<Self>`.
- `TryFromStatus<Self>`.
- A compile-time stable `const TYPE_IDS: &[&str]` listing every variant's `type_id` for testing.

**Attribute summary:**

| On | Attribute | Required | Notes |
|---|---|---|---|
| variant | `category = "..."` | yes | one of `business` / `system` / `validation` / `transport` |
| variant | `code = "..."` | yes | snake_case grpc code |
| variant | `exposure = "..."` | no | overrides category default |
| variant | `error = "..."` | no | thiserror format string for `Display`; default = variant snake_case |
| variant | `from` | no | also emit `From<AnyError>` for this variant (catch-all) |
| field | `#[source]` | no | wires `Error::source` for the chain |
| field | `#[from]` | no | wires `From<T>` for the containing variant |
| field | `#[redact]` | no | strip on egress below `Internal` |

Macro validates at build time:

- Exactly one `code` attribute per variant.
- `System`-category variants forbid `exposure = "public"` (compile error) — server defects must never be marked public-by-default.
- All `type_id`s within a crate are unique (lint, default-warn, configurable to error).

---

## 11. Tonic integration

Two interchangeable integration modes share one encoder/decoder core (`aerro::wire::{encode, decode}`).

### 11.1 Tower layers (`tonic` feature)

```rust
let svc = MyServiceServer::new(impl_)
    .layer(aerro::tower::ServerLayer::new()
        .service_name("create-user")
        .exposure(Exposure::Public)
        .max_frames(8));
```

```rust
let mut client = MyServiceClient::with_interceptor(
    Channel::connect(/* ... */).await?,
    aerro::tower::ClientLayer::new().caller_service("api-gateway"),
);
```

- `ServerLayer<F>` is `tower::Layer`; on response, if the body is `Err(impl Aerro)`, run the encoder; on `Ok`, pass through with zero cost.
- `ClientLayer` is also `tower::Layer`; on response, if `Status::details()` contains an aerro envelope, decode it and attach a frame; otherwise synthesise a `Transport` `RemoteError`.
- Both layers play nicely with `tower-retry`, `tower-timeout`, `tower::ServiceBuilder`, etc.

### 11.2 Macro adapter (`macro` feature)

```rust
#[aerro::handler(service = "create-user", exposure = "public", max_frames = 8)]
async fn create_user(req: Req) -> Result<Resp, CreateUser> { /* ... */ }
```

The macro emits an inline function that wraps the user's body, applies the same encoder, returns `Result<Resp, tonic::Status>` to tonic — no dyn dispatch, no `tower::Service` indirection. Used in services that can't tolerate even the layer's vtable.

**Both modes guarantee:**

- The same `Envelope` bytes for the same input.
- A frame is appended exactly once per hop.
- Exposure is enforced before any wire bytes are written.

### 11.3 Helper extension traits

For manual call sites that want neither layer nor macro:

```rust
trait ResultIntoStatusExt<T, E: Aerro> {
    fn into_status_ext(self, opts: &EncodeOptions) -> Result<T, tonic::Status>;
}
trait StatusIntoResultExt {
    fn into_aerro<E: Aerro>(self) -> Result<ServiceFailure<E>, RemoteError>;
}
```

---

## 12. eyre / anyhow interop

- **`AnyError` alias.** When `anyhow` feature is on, `aerro::AnyError = anyhow::Error`. When `eyre` is on, `aerro::AnyError = eyre::Report`. Mutually exclusive (compile error if both).
- **Inside handlers.** Idiomatic `?` chains using anyhow/eyre. The `Internal(#[from] AnyError)` variant catches anything that doesn't have a typed home, preserving the full `Error::source()` chain.
- **On egress.** At `Internal` exposure, the encoder walks the source chain and renders it into the frame's `message` field (one source per line). At `Trusted` or `Public`, the source chain is dropped and `message` is the variant's `Display` only.
- **Caller side.** `ServiceFailure<E>` and `RemoteError` implement `std::error::Error`, so they nest into a caller's `anyhow::Result<...>` without ceremony.

---

## 13. Performance budget

**Strict, verified in CI.**

- On `Ok(_)` through `ServerLayer`, `ClientLayer`, and the macro adapter: **zero allocations, zero atomic ops, zero map lookups**.
- Frame construction, prost encode, bincode encode, source-chain walking, and `tracing` context capture all live behind `Err(_)` arms.
- Layer state is `Copy` or `&'static`, never `Arc`-cloned per call.
- A `dhat`-based test in `crates/aerro/tests/perf_alloc.rs` asserts zero allocations on a representative `Ok(_)` round-trip through both layers and the macro.
- A `criterion` bench in `crates/aerro/benches/error_path.rs` measures error-path cost and compares it to a baseline that uses `tonic::Status::with_details(serde_json::to_vec(...))` — aerro must be ≤ baseline on encode and decode time.

---

## 14. Testing strategy

| Layer | Mechanism | Coverage |
|---|---|---|
| Unit | per-module `#[cfg(test)] mod tests` | trait impls, encode/decode, redaction, frame elision, attribute validation |
| Integration | `crates/aerro/tests/roundtrip.rs` | spin a real tonic server + client, round-trip every category, every exposure level, with and without `tracing` |
| Polyglot | `crates/aerro/tests/polyglot.rs` | a tonic client built without the `aerro` crate calling an aerro server: must observe correct `Status.code()` + `Status.message()`, may ignore `details()` |
| Compat | `crates/aerro/tests/compat_json.rs` | aerro client decoding a uni-shaped JSON Status, and vice versa, under the `compat-json` feature |
| Perf | `tests/perf_alloc.rs` + `benches/error_path.rs` | the budget asserted in §13 |
| Macro | `crates/aerro-macros/tests/ui/` | `trybuild` UI tests for valid and invalid `#[aerro::operation]` usage |

---

## 15. v1 file plan

```
crates/aerro/
├── Cargo.toml
├── build.rs                        # invokes tonic-build/prost for proto only
├── proto/aerro.v1.proto
└── src/
    ├── lib.rs                      # mod decls + re-exports only
    ├── category.rs
    ├── exposure.rs
    ├── frame.rs
    ├── trace.rs                    # TraceContext, tracing feature glue
    ├── failure.rs                  # ServiceFailure<E>
    ├── remote.rs                   # RemoteError + downcast
    ├── any.rs                      # AnyError alias + chain helpers
    ├── error.rs                    # DecodeError, encode/decode errors
    ├── traits/
    │   ├── mod.rs                  # mod decls only
    │   ├── aerro.rs                # `trait Aerro`
    │   ├── into_status.rs
    │   └── try_from_status.rs
    ├── wire/
    │   ├── mod.rs
    │   ├── envelope.rs             # generated proto re-export + helpers
    │   ├── encode.rs               # writes Envelope into Status.details()
    │   └── decode.rs               # reads Envelope back out
    ├── tower/
    │   ├── mod.rs
    │   ├── server.rs               # ServerLayer + ServerService
    │   └── client.rs               # ClientLayer
    ├── ext.rs                      # ResultIntoStatusExt, StatusIntoResultExt
    └── compat_json.rs              # feature-gated

crates/aerro-macros/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── operation.rs                # #[aerro::operation] enum macro
    ├── handler.rs                  # #[aerro::handler] adapter macro
    ├── attrs.rs                    # attribute parsing
    └── codegen/
        ├── mod.rs
        ├── aerro_impl.rs
        ├── into_status.rs
        ├── try_from_status.rs
        └── thiserror_glue.rs

examples/
├── basic.rs                        # one server, one client, one typed error
├── trace_chain.rs                  # 3-hop chain showing frame accumulation + elision
├── exposure.rs                     # same enum used at Internal / Trusted / Public routes
├── compat.rs                       # round-trips with a uni-style JSON service via compat-json
└── tower_compose.rs                # ServerLayer composed with tower-retry / tower-timeout
```

Module-root files (`lib.rs`, every `mod.rs`) carry only `mod` declarations, `pub use` lines, and crate-level docs.

---

## 16. Verification plan

When the implementation lands, verify the spec end-to-end in this order:

1. `cargo build --workspace --all-features` — clean build.
2. `cargo test --workspace --all-features` — all unit, integration, polyglot, compat, and trybuild tests green.
3. `cargo test --workspace --no-default-features` — minimal-features build still compiles and passes core tests.
4. `cargo bench --bench error_path` — error-path benches ≤ JSON-baseline on encode and decode time.
5. `cargo test --test perf_alloc` — zero-alloc assertion holds.
6. `examples/trace_chain.rs` run manually, observe a 3-hop trace with frames in the correct order and OTel IDs populated.
7. `examples/exposure.rs` run manually with `grpcurl`, verify a Public-exposure route never leaks a `System`-message body.
8. `cargo doc --workspace --no-deps` — docs build without warnings.

---

## 17. Open questions to resolve during implementation (not blockers)

These are deliberately deferred — the spec is approved without resolving them.

- **`type_id` collision strategy across crates.** A workspace-wide lint is in scope; a registry across third-party crates is not. Document the convention `<crate>.<enum>.<variant>` as a recommendation in the README; do not enforce it.
- **`#[track_caller]` propagation through macros.** Verify the macro adapter carries `#[track_caller]` to the encoder call site so locations point at the user's `?`, not at generated code.
- **`tracing` feature without `tonic` feature.** Should we allow `tracing` standalone for users who want the trace-id capture in `IntoStatus` without the tower layer? Lean yes; cost is small.
- **`SmallVec` inline capacity.** 4 is a guess. Re-tune after benches once frame distributions are visible.
- **`tonic-types` interop.** Should `RemoteError` also expose a `google.rpc.Status` view for users who already consume those? Decide during implementation, default no.

---

## 18. Suggested split into multiple spec files

If/when this document grows past the v1 surface, the natural split is:

- `00-context-and-philosophy.md` — §1, §2, §3
- `10-crate-layout.md` — §4, §15
- `20-core-types.md` — §5
- `30-wire-envelope.md` — §6
- `40-trace-model.md` — §7
- `50-categories-and-exposure.md` — §8, §9
- `60-macros.md` — §10
- `70-tonic-integration.md` — §11
- `80-anyhow-eyre.md` — §12
- `90-perf-and-testing.md` — §13, §14
- `99-open-questions.md` — §17

A single-file home is fine for v1; the split is recommended once the spec grows.

---

*End of spec.*
