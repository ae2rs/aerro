# aerro v0.7 — Top 5 Code Review Findings: Implementation Plan

Ordered simplest-to-most-complex within a priority gradient (correctness bugs
before ergonomics before architecture). Each section is one PR.

---

## PR 1: Fix silent encode_payload failure

### PR title

`fix: propagate bincode encode errors instead of silently dropping payload`

### PR description

The macro-generated `encode_payload` uses `if let Ok(bytes) = bincode::encode_to_vec(...)`,
silently swallowing encoding failures and producing an empty payload. The
receiver then gets a `DecodeError::Payload` with no indication that the *sender*
failed — a correctness bug that is extremely hard to diagnose in production.

This PR changes `encode_payload` to return `Result<(), EncodeError>` and
propagates the error up through `encode()` → `IntoStatus`. When bincode
encoding fails, the `tonic::Status` carries `Code::Internal` with a message
identifying the encoding failure, rather than silently producing a corrupt
envelope.

**Tradeoff considered:** Making `encode_payload` fallible changes the `Aerro`
trait signature (breaking). Since we're pre-1.0 and the silent-failure
alternative is a correctness bug, this is the right call. The manual-impl
`Boom` test fixture and any downstream manual impls need updating, but the
derive macro handles it automatically.

### Files to touch

| File | Change |
|------|--------|
| `crates/aerro/src/traits/aerro.rs` | Change `encode_payload` return type to `Result<(), EncodeError>` |
| `crates/aerro/src/error.rs` | Add `Bincode(String)` variant to `EncodeError` (or keep the existing `EncodeError(String)`) |
| `crates/aerro-macros/src/codegen/aerro_impl.rs` | `encode_payload_arm`: replace `if let Ok` with `?` propagation |
| `crates/aerro/src/wire/encode.rs` | `encode()`: handle `encode_payload` error, fall back to `Code::Internal` status |
| `crates/aerro/src/test_support.rs` | Update `Boom::encode_payload` to return `Result` |
| `crates/aerro/tests/roundtrip.rs` | No change needed (uses derive macro) |
| `crates/aerro/tests/smoke.rs` | Update if manual impl exists |

### Implementation steps

1. **Update `Aerro` trait signature** (`crates/aerro/src/traits/aerro.rs:34`):
   Change `fn encode_payload(&self, route: Exposure, buf: &mut Vec<u8>);`
   to `fn encode_payload(&self, route: Exposure, buf: &mut Vec<u8>) -> Result<(), crate::EncodeError>;`

2. **Update macro codegen** (`crates/aerro-macros/src/codegen/aerro_impl.rs`):
   In `encode_payload_arm()` (lines 123-188), replace the `if let Ok` pattern:
   ```rust
   // Before (lines 159-163):
   if let ::core::result::Result::Ok(__bytes) =
       ::bincode::encode_to_vec(&__tup, ::bincode::config::standard())
   {
       __buf.extend_from_slice(&__bytes);
   }

   // After:
   let __bytes = ::bincode::encode_to_vec(&__tup, ::bincode::config::standard())
       .map_err(|e| ::aerro::EncodeError(e.to_string()))?;
   __buf.extend_from_slice(&__bytes);
   ::core::result::Result::Ok(())
   ```
   Also update the unit-variant arm (line 133-137) to return `Ok(())`.

3. **Update `encode()` function** (`crates/aerro/src/wire/encode.rs:33-63`):
   Change line 43 from `sf.inner.encode_payload(route, &mut payload);` to:
   ```rust
   if let Err(e) = sf.inner.encode_payload(route, &mut payload) {
       return Status::new(Code::Internal, format!("aerro: encode failed: {e}"));
   }
   ```

4. **Update `Boom` test fixture** (`crates/aerro/src/test_support.rs:29-31`):
   Change `encode_payload` to return `Result<(), EncodeError>`, wrapping the
   existing `.unwrap()` into a proper `map_err` + `?`.

5. **Add a test for encode failure** (`crates/aerro/src/wire/encode.rs`, test module):
   Create a type whose `encode_payload` always returns `Err(EncodeError(...))` and
   verify that `encode()` produces `Code::Internal` with a message containing
   "encode failed".

### Testing strategy

- `cargo test` — all existing tests must still pass (derive-macro types are
  updated automatically by step 2; manual impls updated in step 4).
- New unit test verifying that a failing `encode_payload` produces a graceful
  `Code::Internal` status instead of an empty/corrupt payload.
- `cargo test --test roundtrip` — the integration test confirms the derive
  macro's new codegen still round-trips correctly.
- Trybuild UI tests unchanged (they test compile errors, not runtime).

### Risks

- **Breaking change to `Aerro` trait.** Any downstream crate with a manual
  `impl Aerro` must update the `encode_payload` signature. Acceptable at 0.x.
- **Performance:** Adding `Result` to `encode_payload` is zero-cost on the
  success path (the `Result` is `Ok` and the `?` is a branch-not-taken).
- **Migration:** Derive-macro users get this for free. Manual impl users change
  one line. Document in CHANGELOG.

---

## PR 2: Add zero-arg `into_status()` convenience path

### PR title

`feat: add default into_status() that uses EncodeOptions::default()`

### PR description

Every call site currently writes `err.into_status(&EncodeOptions::default())`.
This is ceremonial — the vast majority of internal services use the default
(Internal exposure, 16 frames). This PR adds a zero-arg convenience method so
the common case is `err.into_status_default()`.

We do NOT change the existing `IntoStatus` trait (it remains parameterized) —
we add a new free method / trait with a defaulted version. This avoids
ambiguity and keeps the explicit path for services that need non-default options.

**Alternative considered:** Making `EncodeOptions` an `Option<&EncodeOptions>`
parameter. Rejected because it changes the existing trait signature and
`None` is less self-documenting than `into_status_default()`.

**Alternative considered:** A blanket default method on `IntoStatus` itself:
`fn into_status_default(self) -> Status where Self: Sized`. Chosen — this is
the cleanest approach. One new default method on the existing trait, no new
traits, no new types.

### Files to touch

| File | Change |
|------|--------|
| `crates/aerro/src/traits/into_status.rs` | Add default method `into_status_default` to `IntoStatus` trait |
| `crates/aerro/src/lib.rs` | Doc example can optionally use the new method |
| `crates/aerro/examples/basic.rs` | Update to show both paths |

### Implementation steps

1. **Add default method to `IntoStatus`** (`crates/aerro/src/traits/into_status.rs`):
   ```rust
   pub trait IntoStatus {
       fn into_status(self, opts: &EncodeOptions) -> Status;

       fn into_status_default(self) -> Status
       where
           Self: Sized,
       {
           self.into_status(&EncodeOptions::default())
       }
   }
   ```

2. **Update `basic.rs` example** to demonstrate the convenience path:
   ```rust
   let status = err.into_status_default();
   ```

3. **Update lib.rs doc comment** to show the shorter form in the quick example.

4. **Add test** in `crates/aerro/src/ext.rs` test module confirming
   `into_status_default()` produces the same result as
   `into_status(&EncodeOptions::default())`.

### Testing strategy

- `cargo test` — existing tests unaffected (they still use the explicit form).
- New test comparing output of both paths.
- `cargo run --example basic` — verify it compiles and runs.

### Risks

- **Naming:** `into_status_default` is slightly verbose. Alternatives:
  `to_status()`, `as_status()`. But `into_status_default` is unambiguous and
  follows Rust conventions for "same operation, default config."
- **Zero breakage:** This is purely additive. No existing code changes.
- **Orphan rules:** The default method is on the same trait, so no orphan issues.

---

## PR 3: Replace Deref-to-inner with proper accessors

### PR title

`refactor!: replace Deref<Target=Inner> with accessor methods on ServiceFailure and RemoteError`

### PR description

`ServiceFailure<E>` and `RemoteError` both use `Deref<Target = *Inner>` to
expose their boxed fields. This is the "Deref as accessor" anti-pattern:

1. It exposes `ServiceFailureInner` / `RemoteErrorInner` as public API surface
   that users must name in trait bounds and error messages.
2. Auto-deref makes method resolution unpredictable — any method added to the
   inner type silently shadows methods on the outer type.
3. `RemoteErrorParts` is structurally identical to `RemoteErrorInner`, creating
   redundant public types.

This PR:
- Removes `Deref`/`DerefMut` impls from both types.
- Adds explicit accessor methods: `inner()`, `inner_mut()`, `frames()`,
  `frames_mut()`, `trace()`, `trace_mut()` on `ServiceFailure<E>`.
- Adds explicit accessors on `RemoteError`: `category()`, `type_id()`,
  `frames()`, `trace()`, `outer_code()`, `outer_message()`.
- Merges `RemoteErrorParts` into `RemoteErrorInner` (one type, one name).
- Makes `ServiceFailureInner` private (the type no longer needs to be public).

**Tradeoff:** This is a breaking API change. Users currently write `sf.inner`
and `sf.frames` via Deref — they must change to `sf.inner()` and `sf.frames()`.
At 0.x this is acceptable, and the compiler errors are straightforward.

### Files to touch

| File | Change |
|------|--------|
| `crates/aerro/src/failure.rs` | Remove `Deref`/`DerefMut`, add accessors, make `ServiceFailureInner` `pub(crate)` |
| `crates/aerro/src/remote.rs` | Remove `Deref`/`DerefMut`, add accessors, merge `RemoteErrorParts` into `RemoteErrorInner` |
| `crates/aerro/src/wire/encode.rs` | Change `sf.inner.X()` → `sf.inner().X()`, `sf.frames` → `sf.frames()`, `sf.trace` → `sf.trace()` |
| `crates/aerro/src/wire/decode.rs` | Update `RemoteErrorParts` → `RemoteErrorInner`, update field access |
| `crates/aerro/src/ext.rs` | Update test: `sf.inner.x` → `sf.inner().x` |
| `crates/aerro/src/lib.rs` | Update doc example if it uses field access |
| `crates/aerro/tests/roundtrip.rs` | Update `self.inner` → `self.inner()`, `self.frames` → `self.frames()`, `self.trace` → `self.trace()` |
| `crates/aerro/tests/smoke.rs` | Same field access updates |
| `crates/aerro/examples/basic.rs` | Update `recovered.into_inner()` (this already works) |

### Implementation steps

1. **Add accessor methods to `ServiceFailure<E>`** (`crates/aerro/src/failure.rs`):
   ```rust
   pub fn inner(&self) -> &E { &self.state.inner }
   pub fn inner_mut(&mut self) -> &mut E { &mut self.state.inner }
   pub fn frames(&self) -> &SmallVec<[Frame; 4]> { &self.state.frames }
   pub fn frames_mut(&mut self) -> &mut SmallVec<[Frame; 4]> { &mut self.state.frames }
   pub fn trace(&self) -> &TraceContext { &self.state.trace }
   pub fn trace_mut(&mut self) -> &mut TraceContext { &mut self.state.trace }
   ```

2. **Remove `Deref`/`DerefMut` impls** from `ServiceFailure` (lines 69-80).
   Change `ServiceFailureInner` visibility to `pub(crate)`.

3. **Update `Display` and `Error` impls** for `ServiceFailure` to use
   `self.state.inner` directly (they're in the same module, so `pub(crate)` works).

4. **Add accessor methods to `RemoteError`** (`crates/aerro/src/remote.rs`):
   ```rust
   pub fn category(&self) -> Category { self.state.category }
   pub fn type_id(&self) -> &str { &self.state.type_id }
   pub fn frames(&self) -> &SmallVec<[Frame; 4]> { &self.state.frames }
   pub fn trace(&self) -> &TraceContext { &self.state.trace }
   pub fn outer_code(&self) -> Code { self.state.outer_code }
   pub fn outer_message(&self) -> &str { &self.state.outer_message }
   ```

5. **Remove `Deref`/`DerefMut`** from `RemoteError` (lines 75-86).

6. **Merge `RemoteErrorParts` into `RemoteErrorInner`** — delete `RemoteErrorParts`,
   change `from_parts` to accept `RemoteErrorInner` directly.

7. **Update all call sites** in `wire/encode.rs`, `wire/decode.rs`, `ext.rs`,
   tests, examples. The compiler will find them all — just follow the errors.

8. **Update `roundtrip.rs` `CloneForTest`** (lines 90-108): change `self.inner`
   to `self.inner()`, `self.frames` to `self.frames()`, `self.trace` to
   `self.trace()`.

### Testing strategy

- `cargo test` — compiler-driven migration; once it compiles, the behavior is
  identical.
- All existing roundtrip, smoke, polyglot, and UI tests must pass unchanged
  (they test behavior, not API shape).
- Verify `cargo doc` still builds and the public API surface no longer exposes
  `ServiceFailureInner`.

### Risks

- **Breaking change.** Every downstream use of `sf.inner`, `sf.frames`,
  `sf.trace`, `re.type_id`, `re.category`, etc. must change to method calls.
  Compiler errors are clear and mechanical.
- **`DerefMut` loss for `frames.push()`.** Users currently write
  `sf.frames.push(f)` — they must now write `sf.frames_mut().push(f)`. Slightly
  more verbose but explicit.
- **Migration path:** Document in CHANGELOG with before/after examples. Consider
  adding a `#[deprecated]` `Deref` impl for one release cycle, but at 0.x this
  is unnecessary.

---

## PR 4: Make tracing deps optional and eliminate protoc build requirement

### PR title

`refactor: gate tracing/OTel behind feature flag properly, inline proto codegen to remove protoc`

### PR description

Two dependency-weight issues:

**Tracing stack (3 crates for 24 bytes):** `tracing`, `tracing-opentelemetry`,
and `opentelemetry` are already behind the `tracing` feature flag, but it's
on by default. The 24 bytes of `TraceContext` are always present in the
envelope (they're just zeros when the feature is off). This is fine —
the issue is that `default = ["macro", "tracing"]` means every user
pays the compile-time cost unless they opt out. Change the default to
`default = ["macro"]` and document the `tracing` feature clearly.

**protoc build dependency:** `build.rs` calls `tonic_build::compile_protos()`
which requires the `protoc` binary at build time. This is a significant
adoption barrier. The proto file (`aerro.v1.proto`) is stable and changes
rarely. Replace the runtime codegen with a checked-in generated file
(committed `aerro.v1.rs`) and remove the `tonic-build` build dependency
and `build.rs` entirely.

### Files to touch

| File | Change |
|------|--------|
| `crates/aerro/Cargo.toml` | Remove `tracing` from default features; remove `tonic-build` from `[build-dependencies]` |
| `Cargo.toml` (workspace) | Optionally remove `tonic-build` from `[workspace.dependencies]` if unused elsewhere |
| `crates/aerro/build.rs` | Delete entirely |
| `crates/aerro/src/wire/mod.rs` | Change `include!(concat!(env!("OUT_DIR"), ...))` to `include!("generated/aerro.v1.rs")` or inline the module |
| `crates/aerro/src/wire/generated/aerro.v1.rs` | New file — checked-in output of `tonic_build` / `prost-build` |
| `crates/aerro/src/lib.rs` | Update feature flag docs table |
| `README.md` | Remove any "requires protoc" note; document `tracing` feature |

### Implementation steps

1. **Generate and check in the proto output:**
   - Run `cargo build` once to produce `target/debug/build/aerro-*/out/aerro.v1.rs`.
   - Copy that file to `crates/aerro/src/wire/generated/aerro.v1.rs`.
   - Add a header comment: `// Generated from proto/aerro.v1.proto — do not edit manually.`
   - Add a `// Re-generate: tonic-build + prost-build, see proto/README.md`

2. **Update `wire/mod.rs`** (line 3-5):
   ```rust
   pub mod raw {
       include!("generated/aerro.v1.rs");
   }
   ```

3. **Delete `crates/aerro/build.rs`.**

4. **Remove `tonic-build` from `Cargo.toml`:**
   - Remove `[build-dependencies]` section from `crates/aerro/Cargo.toml`.
   - Remove `tonic-build` from workspace deps if no other crate uses it.

5. **Change default features** in `crates/aerro/Cargo.toml`:
   ```toml
   [features]
   default = ["macro"]
   ```

6. **Update lib.rs doc table** to note `tracing` is opt-in:
   ```
   | `tracing` | ✗ | Capture OTel trace/span IDs via the `tracing` subscriber |
   ```

7. **Add a `proto/README.md`** with regeneration instructions:
   ```
   To regenerate: install protoc, then run:
     tonic-build ... (exact command)
   Copy output to src/wire/generated/aerro.v1.rs
   ```

8. **Verify build works without protoc:**
   ```
   PATH=/usr/bin:/bin cargo build  # (stripped PATH, no protoc)
   ```

### Testing strategy

- `cargo test` with default features (no `tracing`) — all non-tracing tests pass.
- `cargo test --features tracing` — tracing tests pass.
- `cargo build` on a system without `protoc` installed — must succeed.
- `cargo test --test polyglot` — proto interop test still passes with the
  checked-in generated code.

### Risks

- **Default feature removal (`tracing`):** Downstream crates that depend on
  `aerro` and use `TraceContext::capture()` to get real trace IDs must now
  add `features = ["tracing"]`. Document in CHANGELOG.
- **Stale generated code:** If `aerro.v1.proto` is updated but the generated
  file isn't regenerated, they drift. Mitigate with a CI check:
  `diff <(regenerate) src/wire/generated/aerro.v1.rs`.
- **Generated file size:** The prost output for this small proto is ~100 lines.
  Trivial to review in PRs.

---

## PR 5: Resolve hybrid wire format — choose all-protobuf or all-bincode

### PR title

`refactor!: unify wire format to pure bincode, removing protobuf envelope`

### PR description

The current wire format wraps a protobuf `Envelope` (parseable by any language)
around a bincode `payload` (Rust-only). This is philosophically incoherent:
the protobuf envelope promises polyglot interop, but the payload — the actual
error data — is opaque to non-Rust consumers. The proto facade provides no
real value while imposing `prost` and `protoc` as dependencies.

**Decision: all-bincode.** Rationale:
- aerro is a Rust-to-Rust error framework. The README, examples, and API
  surface are all Rust-specific (derive macros, `thiserror` integration,
  `tonic::Code`).
- The envelope fields (category, type_id, trace_id, span_id, frames, version)
  are simple and map directly to bincode-serializable Rust structs.
- Removing prost eliminates `prost`, `tonic-build`, and `protoc` as
  dependencies, cutting compile time significantly.
- If polyglot support is needed in the future, it should be a proper
  `serde`-based encoding behind a feature flag, not a half-proto/half-bincode
  hybrid.

**Alternative considered: all-protobuf.** Would require defining the payload
schema in proto (e.g., `google.protobuf.Any` or a `map<string, Value>`). This
loses the zero-copy bincode performance and requires every error variant to
define a proto message. Rejected because aerro's value prop is Rust ergonomics,
not polyglot.

**Wire version bump:** The envelope `version` field moves from 1 to 2. The
decoder will reject version-1 envelopes with `DecodeError::UnsupportedVersion`
unless the `compat-v1` feature flag is enabled (reads both, writes v2 only).

### Files to touch

| File | Change |
|------|--------|
| `crates/aerro/src/wire/mod.rs` | Remove `raw` module (prost-generated code) |
| `crates/aerro/src/wire/envelope.rs` | Rewrite: define `WireEnvelope` as a bincode-serializable struct |
| `crates/aerro/src/wire/encode.rs` | Encode `WireEnvelope` via bincode instead of prost |
| `crates/aerro/src/wire/decode.rs` | Decode `WireEnvelope` via bincode instead of prost |
| `crates/aerro/src/frame.rs` | Derive `bincode::Encode`/`Decode` on `Frame` (or a wire-specific `WireFrame`) |
| `crates/aerro/src/category.rs` | Derive `bincode::Encode`/`Decode` on `Category` |
| `crates/aerro/src/trace.rs` | Derive `bincode::Encode`/`Decode` on `TraceContext` |
| `crates/aerro/Cargo.toml` | Remove `prost` from `[dependencies]`; remove `tonic-build` from `[build-dependencies]` |
| `Cargo.toml` (workspace) | Remove `prost`, `tonic-build` from workspace deps |
| `crates/aerro/build.rs` | Delete (already deleted in PR 4, but listed for completeness) |
| `crates/aerro/proto/aerro.v1.proto` | Move to `proto/archive/` or delete; add deprecation note |
| `crates/aerro/tests/polyglot.rs` | Rewrite or delete (polyglot interop no longer claimed) |
| `crates/aerro/tests/proto_gen.rs` | Delete (no longer applicable) |

### Implementation steps

1. **Define `WireEnvelope` struct** (`crates/aerro/src/wire/envelope.rs`):
   ```rust
   #[derive(bincode::Encode, bincode::Decode)]
   pub(crate) struct WireEnvelope {
       pub version: u32,
       pub category: u8,
       pub type_id: String,
       pub trace_id: [u8; 16],
       pub span_id: [u8; 8],
       pub frames: Vec<WireFrame>,
       pub payload: Vec<u8>,
   }

   #[derive(bincode::Encode, bincode::Decode)]
   pub(crate) struct WireFrame {
       pub service: String,
       pub rpc: String,
       pub code: u32,
       pub message: String,
       pub location: String,
       pub category: u8,
   }
   ```

2. **Add `Category ↔ u8` conversions** (`crates/aerro/src/category.rs`):
   Replace the `to_proto`/`from_proto` helpers in `envelope.rs` with direct
   `From<Category> for u8` / `TryFrom<u8> for Category`.

3. **Rewrite `encode()`** (`crates/aerro/src/wire/encode.rs`):
   - Build a `WireEnvelope` from the `ServiceFailure<E>`.
   - Serialize with `bincode::encode_to_vec(&env, bincode::config::standard())`.
   - Put the bytes into `Status::with_details(...)`.

4. **Rewrite `decode()`** (`crates/aerro/src/wire/decode.rs`):
   - `bincode::decode_from_slice::<WireEnvelope>(details, ...)`.
   - Check `version == 2`.
   - Reconstruct `ServiceFailure<E>` or `RemoteError`.

5. **Remove prost dependency:**
   - Delete `crates/aerro/src/wire/generated/` (or `raw` module).
   - Remove `prost` from `crates/aerro/Cargo.toml`.
   - Remove `build.rs` if not already deleted.
   - Remove `proto/aerro.v1.proto` or archive it.

6. **Update `wire/mod.rs`:**
   ```rust
   pub mod decode;
   pub mod encode;
   pub mod envelope;
   // raw module removed
   ```

7. **Delete `tests/proto_gen.rs`** and rewrite `tests/polyglot.rs`
   (polyglot test was verifying proto-parsability; replace with a version-check
   test ensuring v2 envelopes decode correctly and v1 envelopes produce
   `UnsupportedVersion`).

8. **Bump `ENVELOPE_VERSION` to 2** in `envelope.rs`.

9. **(Optional) Add `compat-v1` feature flag** that enables a fallback decoder
   accepting version-1 (prost) envelopes during rolling upgrades. This would
   keep `prost` as an optional dep behind the flag.

10. **Update all tests and examples.** Most changes are in the wire layer;
    the `Aerro` trait and macro codegen are unaffected since they already
    produce bincode.

### Testing strategy

- `cargo test` — all existing roundtrip and smoke tests must pass with the
  new wire format.
- New test: encode with v2, decode with v2 — round-trip correctness.
- New test: attempt to decode a v1-era envelope → `DecodeError::UnsupportedVersion`.
- Benchmark: `cargo bench --bench error_path` — compare encode/decode latency
  before and after (expect improvement: one fewer serialization layer).
- `cargo build` — verify `prost` no longer appears in `Cargo.lock`.
- `cargo tree -d` — no duplicate crates from prost/tonic-build.

### Risks

- **Wire-incompatible with v1.** Any service running aerro <=0.6 cannot
  decode envelopes from aerro >=0.7 and vice versa. This is expected for a
  0.x crate, but rolling upgrades in production require coordination.
  Mitigate with the `compat-v1` feature flag for the transition period.
- **Loss of polyglot narrative.** The proto file was a selling point for
  "any language can parse the envelope." Dropping it means aerro is explicitly
  Rust-only. This should be reflected in README and crate description.
- **Bincode stability.** bincode 2.x has a stable format, but it's less
  standardized than protobuf. If wire stability is a hard requirement, pin
  `bincode = "=2.0.X"` and add a golden-file test (encode a known struct,
  compare bytes to a committed snapshot).
- **Complexity.** This is the largest change in the series. It should be done
  last, after PRs 1-4 are merged, so the codebase is cleaner when the wire
  format changes.
- **tonic dependency remains.** We still depend on `tonic` for `Status`,
  `Code`, and the gRPC transport. Only `prost` and `tonic-build` are removed.

---

## Dependency graph between PRs

```
PR 1 (encode_payload error)     ── independent
PR 2 (into_status_default)      ── independent
PR 3 (Deref removal)            ── independent (but benefits from PR 1 being merged first
                                    so the Aerro trait only changes once)
PR 4 (dep weight)               ── independent
PR 5 (wire format)              ── depends on PR 4 (protoc removal is subsumed)
```

**Recommended merge order:** PR 1 → PR 2 → PR 3 → PR 4 → PR 5

PRs 1 and 2 can be merged in parallel (they touch disjoint code). PR 3
should follow PR 1 so that `Aerro` trait changes are batched. PR 4 is a
prerequisite for PR 5 (PR 5 subsumes the protoc removal from PR 4, but
the tracing default-feature change in PR 4 is independent).

---

## Version strategy

- PRs 1-2: patch bump → **0.6.3** (bugfix + additive feature)
- PR 3: minor bump → **0.7.0** (breaking API change)
- PR 4: minor bump → **0.8.0** (breaking default-feature change)
- PR 5: minor bump → **0.9.0** (breaking wire format change)

Alternatively, batch PRs 3-5 into a single **0.7.0** release if shipping
them close together. The `compat-v1` feature flag in PR 5 gives
downstream users a migration path.
