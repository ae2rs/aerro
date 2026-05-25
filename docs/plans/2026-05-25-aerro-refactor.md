# aerro Refactor — Strip Framework, Keep Wire Format

> **For Hermes:** Use this plan to execute the refactoring PR-by-PR. Each PR is a self-contained branch with CI verification and a release.

**Goal:** Strip aerro of its framework pretensions (Tower middleware, JSON compat, AerroHandler derive, anyhow/eyre bridges) and refocus it as a **wire format + derive macro** for typed gRPC errors.

**Architecture:** Each PR removes one feature/module entirely, keeping the crate building and all remaining tests passing. CI validates every PR before merge. Each merge triggers a GitHub + crates.io release.

**Workflow (repeat for every PR):**
1. Cut a branch from `main`
2. Make changes, commit
3. Push, create PR with `gh`, document it
4. Wait for CI to pass
5. Squash merge, delete branch
6. Create a GitHub release + `cargo publish`

---

## Release Automation Question

Before the first PR, let's answer your question:

> Should we make this automated in the CI? Is it possible without leaking the crates.io token?

**Yes, absolutely possible.** The industry-standard approach:

- Store `CARGO_REGISTRY_TOKEN` as a [GitHub Actions secret](https://docs.github.com/en/actions/security-guides/using-secrets-in-github-actions) — encrypted at rest, never printed in logs, only accessible to Actions runners
- Add a `release` job to the CI workflow that:
  - Runs **only** on push to `main` (after a merge)
  - Detects the version from `Cargo.toml` or a git tag
  - Runs `cargo publish`
  - Creates a GitHub Release via `gh release create`

**But — careful about ordering.** If you publish to crates.io before CI finishes on the merge commit, you could publish broken code. Better flow:

1. PR merges → CI runs on `main` (fmt + clippy + test)
2. On success → a separate `release` workflow publishes to crates.io + GitHub

This is the pattern used by `serde`, `tokio`, `axum`, etc.

I've included an optional final PR to set this up.

---

## PR 1: Strip Tower middleware

**Branch:** `refactor/remove-tower`

**Objective:** Remove all Tower integration code. Tower layers (`ClientLayer`, `ServerLayer`, `ClientService`, `ServerService`) are placeholders that carry config but don't actually intercept requests. The crate doesn't need them.

### Exacting Changes

**Delete these files:**
- `crates/aerro/src/tower/` (entire directory: `mod.rs`, `client.rs`, `server.rs`)
- `crates/aerro/examples/tower_compose.rs`

**Edit `crates/aerro/src/lib.rs`:**
```diff
- pub mod tower;
```

**Edit `crates/aerro/Cargo.toml`:**
```diff
- tower   = { workspace = true }
- http    = { workspace = true }
- pin-project-lite = { workspace = true }
```

**Edit `Cargo.toml` (workspace root):**
```diff
- http = "1"
- tower = { version = "0.5", default-features = false, features = ["util"] }
- pin-project-lite = "0.2"
```

**Edit `crates/aerro/examples/handler.rs`:**
- Remove any `use aerro::tower::*` if present (the handler example uses `AerroHandler`, not Tower directly; the Tower example was `tower_compose.rs` which we're deleting)

**Update CI?** No CI changes needed — Tower removal doesn't affect the test matrix.

### Tests that must still pass
```
cargo test --workspace
cargo test --workspace --features compat-json
cargo test --workspace --features compat-json,eyre
```

These will run in CI. Locally we can't run them without protoc, but CI handles that.

### Release
After merge: bump version to `0.3.0`, tag, `cargo publish`, `gh release create`.

---

## PR 2: Drop compat-json feature

**Branch:** `refactor/remove-compat-json`

**Objective:** Remove the JSON wire envelope alternative. It's intentionally lossy (typed payloads aren't carried), which defeats the crate's purpose. Users who need JSON interop can decode the prost envelope in their own code.

### Exacting Changes

**Delete these files:**
- `crates/aerro/src/compat_json.rs`

**Edit `crates/aerro/src/lib.rs`:**
```diff
- #[cfg(feature = "compat-json")]
- pub mod compat_json;
```

**Edit `crates/aerro/Cargo.toml`:**
```diff
- compat-json = ["dep:serde", "dep:serde_json"]
```
And remove `serde` and `serde_json` from `[dependencies]`.

**Edit `Cargo.toml` (workspace root):**
```diff
- serde = { version = "1", features = ["derive"] }
- serde_json = "1"
```

**Delete file:**
- `crates/aerro/examples/compat.rs`

**Edit `.github/workflows/ci.yml`:**
Remove these matrix entries:
```diff
- - name: compat-json
-   run: cargo clippy --workspace --all-targets --features compat-json -- -D warnings
- - name: eyre + compat-json
-   run: cargo clippy --workspace --all-targets --features compat-json,eyre -- -D warnings
```
```diff
- - name: compat-json
-   run: cargo test --workspace --features compat-json
- - name: eyre + compat-json
-   run: cargo test --workspace --features compat-json,eyre
```

### Tests that must still pass
```
cargo test --workspace
```

### Release
Version `0.4.0`, tag, publish, release.

---

## PR 3: Remove AerroHandler derive macro + handler module

**Branch:** `refactor/remove-aerro-handler`

**Objective:** Remove the `AerroHandler` derive macro and the `Handler`/`AerroHandler` traits. These conflate error encoding with handler dispatch and belong in user code, not in a wire format crate. The pattern is ~15 lines of user-level boilerplate — not worth a derive macro.

### Exacting Changes

**Delete these files:**
- `crates/aerro/src/handler.rs`

**Edit `crates/aerro/src/lib.rs`:**
```diff
- pub mod handler;
- pub use handler::{AerroHandler, Handler};
```
And:
```diff
- #[cfg(feature = "macro")]
- pub use aerro_macros::{Aerro, AerroHandler};
+ #[cfg(feature = "macro")]
+ pub use aerro_macros::Aerro;
```

**Edit `crates/aerro-macros/src/lib.rs`:**
```diff
- mod handler_derive;
```
Remove the `AerroHandler` derive function entirely:
```diff
- #[proc_macro_derive(AerroHandler, attributes(aerro))]
- pub fn aerro_handler_derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
-     handler_derive::expand(item.into()).into()
- }
```

**Delete file:**
- `crates/aerro-macros/src/handler_derive.rs`

**Delete file:**
- `crates/aerro/examples/handler.rs`

**Edit CI?** No CI changes needed.

**Check `crates/aerro/tests/macro_handler.rs`:** Delete this test file too (tests for `AerroHandler`).

### Tests that must still pass
```
cargo test --workspace
```

### Release
Version `0.5.0`, tag, publish, release.

---

## PR 4: Strip anyhow/eyre features

**Branch:** `refactor/remove-anyhow-eyre`

**Objective:** Remove the `anyhow`/`eyre` feature flags and `AnyError` type alias. They add feature flags and CI matrix entries for ~3 lines of actual code (`pub type AnyError = anyhow::Error`). Keep `render_chain()` as a free utility function — it's genuinely useful for displaying error chains.

### Exacting Changes

**Edit `crates/aerro/src/any.rs`:**
Replace the entire file with just the `render_chain` function. Remove `AnyError` and the `anyhow`/`eyre` imports/type aliases.

Delete everything up to `render_chain`, keeping only:
```rust
//! Error chain rendering utility.

/// Render an error and its `source()` chain into `"msg: src1: src2: ..."` form.
pub fn render_chain(err: &(dyn std::error::Error + 'static)) -> String {
    let mut out = err.to_string();
    let mut cur = err.source();
    while let Some(s) = cur {
        out.push_str(": ");
        out.push_str(&s.to_string());
        cur = s.source();
    }
    out
}

#[cfg(test)]
mod tests {
    // ... existing tests, unchanged ...
}
```

**Edit `crates/aerro/Cargo.toml`:**
```diff
- anyhow    = ["dep:anyhow"]
- eyre      = ["dep:eyre"]
```
And remove `anyhow` and `eyre` from `[dependencies]`.

**Edit `Cargo.toml` (workspace root):**
```diff
- anyhow = "1"
- eyre = "0.6"
```

**Edit `crates/aerro/src/lib.rs`:**
```diff
- pub use any::AnyError;
  pub use any::render_chain;
```
Remove the `#[cfg(any(feature = "anyhow", feature = "eyre"))]` gate:
```diff
- #[cfg(any(feature = "anyhow", feature = "eyre"))]
- pub use any::AnyError;
  pub use any::render_chain;
```

**Edit `.github/workflows/ci.yml`:**
Remove these clippy entries:
```diff
- - name: eyre
-   run: cargo clippy --workspace --all-targets --no-default-features --features eyre -- -D warnings
- - name: eyre + compat-json
-   run: cargo clippy --workspace --all-targets --features compat-json,eyre -- -D warnings
```
Remove these test entries:
```diff
- - name: eyre + compat-json
-   run: cargo test --workspace --features compat-json,eyre
```

(Note: the `eyre + compat-json` entries may already be gone after PR 2 — check for cleanup.)

### Tests that must still pass
```
cargo test --workspace
```

### Release
Version `0.6.0`, tag, publish, release.

---

## PR 5 (Optional): Release automation + CI polish

**Branch:** `chore/release-automation`

**Objective:** Add automated publishing to crates.io and GitHub Releases whenever a merge to `main` succeeds. This is the setup described in the "Release Automation" section above.

### Changes

**Create `.github/workflows/release.yml`:**
```yaml
name: Release

on:
  push:
    branches: [main]

jobs:
  release:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: arduino/setup-protoc@v3
        with:
          repo-token: ${{ secrets.GITHUB_TOKEN }}
      - uses: dtolnay/rust-toolchain@stable

      # Check that the crate actually builds before publishing
      - name: Build
        run: cargo build --workspace

      # Publish to crates.io
      - name: Publish to crates.io
        run: cargo publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      # Create a GitHub Release
      - name: Create GitHub Release
        run: |
          VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
          gh release create "v$VERSION" \
            --title "v$VERSION" \
            --notes "See [CHANGELOG](https://github.com/ae2rs/aerro/commits/v$VERSION) for changes."
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Note:** This requires:
1. A `CARGO_REGISTRY_TOKEN` secret added in GitHub repo settings → Secrets and variables → Actions → New repository secret. Generate a token at https://crates.io/settings/tokens.
2. A version bump in `Cargo.toml` before each merge (or the workflow reads the current version and publishes it).

### Trade-offs / discussion

**Pros of automated releases:**
- Eliminates manual steps
- Never forgets to publish
- Tags are consistent
- Works great for `0.x` churn

**Cons / risks:**
- A broken merge to `main` publishes broken code to crates.io
- Fix: make the release workflow depend on CI passing first (use `workflow_run` or merge queue)
- The crates.io token is a secret — GitHub Actions encrypts it, but you still need to trust GitHub's security model

**Alternative:** Semi-automated. The workflow creates a draft GitHub Release only, and you manually press "Publish" + `cargo publish` from your machine. This gives a human-in-the-loop check without leaking the token.

### No release for this PR itself (it's infra)
If you want a release, version `0.6.1`, otherwise skip.

---

## Summary of all PRs

| # | Branch | What changes | Version |
|---|--------|-------------|---------|
| 1 | `refactor/remove-tower` | Delete `tower/`, remove `tower`, `http`, `pin-project-lite` deps | 0.3.0 |
| 2 | `refactor/remove-compat-json` | Delete `compat_json.rs`, remove serde/serde_json, clean CI matrix | 0.4.0 |
| 3 | `refactor/remove-aerro-handler` | Delete `handler.rs`, `handler_derive.rs`, `AerroHandler` macro, examples | 0.5.0 |
| 4 | `refactor/remove-anyhow-eyre` | Strip anyhow/eyre features, keep `render_chain` only, clean CI | 0.6.0 |
| 5 | `chore/release-automation` | Add GitHub Actions release workflow (optional) | 0.6.1 |

**Each PR produces a working crate** — no phase depends on the next being complete. You could stop after PR 1–3 if you want.
