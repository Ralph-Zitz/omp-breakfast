# OpenAPI Sync Validation

Validate that the OpenAPI/Swagger UI spec is fully synchronized with the API route definitions.

## Instructions

You are a REST API auditor ensuring the Swagger UI documentation exactly matches the implemented routes. Compare `src/routes.rs`, `src/middleware/openapi.rs`, and all handler files to find any discrepancies.

### Analysis steps

1. **Route inventory** — Parse `src/routes.rs` and list every registered endpoint: HTTP method, path, and handler function name
2. **OpenAPI inventory** — Parse `src/middleware/openapi.rs` `#[openapi(paths(...))]` and list every handler registered in the spec
3. **Handler annotations** — For each handler in `src/handlers/`, check whether it has a `#[utoipa::path(...)]` annotation
4. **Cross-reference** — Compare the three lists to find:
   - **Routes missing from OpenAPI** — endpoints in `routes.rs` not listed in `openapi.rs` paths
   - **OpenAPI entries missing from routes** — paths in `openapi.rs` not registered in `routes.rs`
   - **Handlers without utoipa annotations** — handler functions referenced in routes that lack `#[utoipa::path]`
   - **Path mismatches** — `#[utoipa::path(path = "...")]` values that don't match actual route paths
   - **Method mismatches** — `#[utoipa::path(get/post/...)]` that don't match the HTTP method in `routes.rs`
5. **Schema coverage** — Check that all request/response types used in handlers are listed in `components(schemas(...))` in `openapi.rs`
6. **Security annotations** — Verify that endpoints behind auth middleware have matching `security(...)` in their utoipa annotations

### Output format

Provide:

1. **Sync status table:**

   | Handler | Route Path | Method | In routes.rs | In openapi.rs | Has #[utoipa::path] | Status |
   | ------- | ---------- | ------ | ------------ | ------------- | ------------------- | ------ |

2. **Issues found** — List each mismatch with the exact file and line to fix

3. **Missing schemas** — Any request/response types not in `components(schemas(...))`

4. **Recommended fixes** — Specific code changes to bring everything into sync
