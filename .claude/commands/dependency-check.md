# Dependency Check

Analyze project dependencies for health, freshness, and optimization opportunities.

## Instructions

You are a Rust build engineer. Review both `Cargo.toml` (backend) and `frontend/Cargo.toml` (Leptos WASM frontend) and assess the health of all dependencies.

### Analysis steps

1. **Dependency inventory** — List every dependency across both crates with its current version and what it's used for
2. **Feature audit** — For each dependency with enabled features, check:
   - Are all enabled features actually used in the code?
   - Are there missing features that should be enabled?
   - Could feature flags reduce compile time?
3. **Redundancy check** — Are there crates that overlap in functionality? E.g.:
   - `eyre` + `color-eyre` + `thiserror` — are all three needed?
   - Multiple TLS-related crates — is the dependency tree minimal?
   - Shared crates between backend and frontend — are versions aligned?
4. **Maintenance status** — Flag any crates that:
   - Haven't been updated in over a year
   - Have known deprecation notices
   - Have been superseded by newer alternatives
5. **Compile time impact** — Identify heavy dependencies (proc macros, C bindings like `aws-lc-sys`) and suggest lighter alternatives if they exist
6. **Dev dependencies** — Are `[dev-dependencies]` sufficient? Are there missing test utilities?
7. **WASM compatibility** — For `frontend/Cargo.toml`:
   - Are all dependencies `wasm32-unknown-unknown` compatible?
   - Is `wasm-bindgen` version aligned with `wasm-pack` expectations?
   - Are `web-sys` features minimal (only needed APIs enabled)?
   - Is `gloo-net` the best choice or should it be replaced (e.g., with `reqwest` + wasm feature)?

### Commands to run

If available, run:

```bash
cargo tree --depth 1
cd frontend && cargo tree --depth 1 --target wasm32-unknown-unknown
```

### Output format

Provide:

1. **Dependency health table** (one for each crate) — Crate | Version | Purpose | Status | Notes
2. **Cross-crate alignment** — Any version mismatches between shared dependencies
3. **Optimization suggestions** — Crates to remove, replace, or reconfigure
4. **Missing dependencies** — Useful crates the project should consider adding (e.g., `cargo-audit`, `cargo-deny`, `cargo-outdated`)
5. **Action items** — Prioritized list of dependency changes to make

### Scope

Read `Cargo.toml`, `frontend/Cargo.toml`, `frontend/Trunk.toml`, and relevant source files to verify usage. Do NOT modify any files — this is analysis only.
