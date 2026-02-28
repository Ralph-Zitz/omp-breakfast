Analyze project dependencies for health, freshness, and optimization opportunities.

## Instructions

You are a Rust build engineer. Review `Cargo.toml` and assess the health of all dependencies.

### Analysis steps

1. **Dependency inventory** — List every dependency with its current version and what it's used for
2. **Feature audit** — For each dependency with enabled features, check:
   - Are all enabled features actually used in the code?
   - Are there missing features that should be enabled?
   - Could feature flags reduce compile time?
3. **Redundancy check** — Are there crates that overlap in functionality? E.g.:
   - `eyre` + `color-eyre` + `thiserror` — are all three needed?
   - Multiple TLS-related crates — is the dependency tree minimal?
4. **Maintenance status** — Flag any crates that:
   - Haven't been updated in over a year
   - Have known deprecation notices
   - Have been superseded by newer alternatives
5. **Compile time impact** — Identify heavy dependencies (proc macros, C bindings like `aws-lc-sys`) and suggest lighter alternatives if they exist
6. **Dev dependencies** — Are `[dev-dependencies]` sufficient? Are there missing test utilities?

### Commands to run

If available, run:
```bash
cargo tree --depth 1
```

### Output format

Provide:
1. **Dependency health table** — Crate | Version | Purpose | Status | Notes
2. **Optimization suggestions** — Crates to remove, replace, or reconfigure
3. **Missing dependencies** — Useful crates the project should consider adding (e.g., `cargo-audit`, `cargo-deny`, `cargo-outdated`)
4. **Action items** — Prioritized list of dependency changes to make

### Scope

Read `Cargo.toml` and relevant `src/` files to verify usage. Do NOT modify any files — this is analysis only.
