# Security Audit

Perform a security audit of the codebase focusing on authentication, data handling, and common web vulnerabilities.

## Instructions

You are a security engineer reviewing a Rust web API with a Leptos WebAssembly frontend. Examine the entire `src/` directory, `frontend/src/` directory, `config/` files, all files in `migrations/`, `database_seed.sql`, `docker-compose.yml`, and `Dockerfile.*` for security issues.

### Areas to audit — Backend

1. **Authentication & Authorization**
   - JWT implementation: algorithm choice, secret strength validation, token lifetime, refresh token rotation
   - Basic auth: password hashing parameters (Argon2 config), timing attacks
   - Token blacklist: is it persisted? What happens on server restart?
   - Are all protected endpoints actually behind auth middleware?

2. **Input validation**
   - SQL injection: are all queries parameterized? Any string interpolation in SQL?
   - Path traversal: are path parameters validated before use?
   - Request size limits: are JSON payload sizes bounded?
   - Are UUIDs validated before DB queries?

3. **Secrets management**
   - Are secrets hardcoded in config files or code?
   - Are secrets logged anywhere (tracing calls)?
   - Is the JWT secret sufficiently random/long?

4. **TLS configuration**
   - Are TLS versions and cipher suites appropriate?
   - Is the DB connection encrypted?
   - Certificate validation: is it strict or does it allow self-signed?

5. **Dependencies**
   - Run `cargo audit --ignore RUSTSEC-2023-0071` if available, or review `Cargo.toml` for known-vulnerable crate versions (the ignore flag acknowledges the unfixable `rsa` timing side-channel via `jsonwebtoken` — see finding #132; re-check periodically whether an upstream fix is available)
   - Are there unnecessary dependencies that increase attack surface?

6. **Information disclosure**
   - Do error responses leak internal details (stack traces, SQL errors, file paths)?
   - Are DB error codes returned to clients?

7. **Docker/deployment**
   - Does the container run as root?
   - Are secrets passed via environment variables or baked into images?

### Areas to audit — Frontend (`frontend/src/`)

1. **Token storage**
   - Is `sessionStorage` the right choice for JWT storage? (vs. HttpOnly cookies or in-memory only)
   - Is the token cleared on logout? On tab close? On session expiry?
   - Could an XSS attack read the token from `sessionStorage`?

2. **Cross-Site Scripting (XSS)**
   - Are user-supplied values (usernames, team names) rendered safely or is raw HTML insertion used?
   - Does Leptos's `view!` macro auto-escape output? Are there any uses of `inner_html` or similar?
   - Are error messages from the server displayed without sanitization?

3. **Client-side auth security**
   - Is the JWT decoded without signature verification on the client? (Expected for display purposes, but must not be trusted for authorization decisions)
   - Are auth headers sent only over HTTPS?
   - Is the `Authorization` header sent to the correct origin only? (Could leaked Trunk proxying send credentials to wrong host?)

4. **CORS & API origin**
   - Is the backend configured with appropriate CORS headers?
   - Could the frontend inadvertently send requests to a different origin?
   - Are fetch requests using `credentials: "same-origin"` or `"include"` appropriately?

5. **Input sanitization**
   - Is client-side validation sufficient, or could a user bypass it (e.g., via browser devtools)?
   - Are there length limits on frontend form inputs to prevent abuse?

6. **Dependency supply chain (frontend)**
   - Are `wasm-bindgen`, `gloo-net`, `web-sys` versions up to date?
   - Could WASM binary be tampered with in transit? (Subresource Integrity / CSP headers)

### Output format

For each finding:

- **Severity:** Critical / High / Medium / Low / Informational
- **Location:** File and line(s)
- **Description:** The vulnerability or concern
- **Impact:** What could go wrong
- **Remediation:** Specific fix with code if applicable

Group findings into **Backend** and **Frontend** sections. End with a risk summary and top 5 fixes to prioritize.

### Scope

Read all project files including `frontend/src/`, `frontend/Trunk.toml`, and `frontend/index.html`. Do NOT modify any files — this is analysis only.
