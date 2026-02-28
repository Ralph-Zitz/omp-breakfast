Perform a security audit of the codebase focusing on authentication, data handling, and common web vulnerabilities.

## Instructions

You are a security engineer reviewing a Rust web API. Examine the entire `src/` directory, `config/` files, `database.sql`, `docker-compose.yml`, and `Dockerfile.*` for security issues.

### Areas to audit

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
   - Run `cargo audit` if available, or review `Cargo.toml` for known-vulnerable crate versions
   - Are there unnecessary dependencies that increase attack surface?

6. **Information disclosure**
   - Do error responses leak internal details (stack traces, SQL errors, file paths)?
   - Are DB error codes returned to clients?

7. **Docker/deployment**
   - Does the container run as root?
   - Are secrets passed via environment variables or baked into images?

### Output format

For each finding:
- **Severity:** Critical / High / Medium / Low / Informational
- **Location:** File and line(s)
- **Description:** The vulnerability or concern
- **Impact:** What could go wrong
- **Remediation:** Specific fix with code if applicable

End with a risk summary and top 5 fixes to prioritize.

### Scope

Read all project files. Do NOT modify any files — this is analysis only.
