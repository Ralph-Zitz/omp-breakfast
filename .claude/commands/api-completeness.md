# API Completeness

Analyze API completeness by comparing the database schema against implemented endpoints, and cross-reference with frontend consumption.

## Instructions

You are an API architect reviewing a REST API for completeness. Compare all migration files in `migrations/` (the authoritative schema) against the implemented handlers, routes, models, and DB functions to identify missing functionality. Also check which endpoints the frontend actually consumes.

### Analysis steps

1. **Schema inventory** — List every table, its columns, relationships (foreign keys), and purpose
2. **API inventory** — List every implemented endpoint with its HTTP method, path, handler function, and what DB function it calls
3. **Gap analysis** — For each table, check:
   - Does it have a corresponding Rust model struct?
   - Does it have CRUD functions in `db/`?
   - Does it have handler functions in `handlers/`?
   - Does it have routes in `routes.rs`?
   - Is it documented in the OpenAPI spec (`middleware/openapi.rs`)?
4. **Stub audit** — Identify all handlers returning `NotImplemented` and map them to the DB tables they should operate on
5. **Relationship coverage** — Are join queries (memberof, team orders) fully exposed via the API?
6. **Missing endpoints** — Suggest endpoints that should exist based on the schema but don't
7. **Frontend consumption** — Read `frontend/src/app.rs` and list every API call the frontend makes:
   - Which endpoints does the frontend call? (e.g., `POST /auth`, `GET /api/v1.0/users/{id}`)
   - Which endpoints exist in the backend but are NOT consumed by the frontend yet?
   - Are there frontend features waiting on unimplemented backend endpoints?
   - Does the frontend expect response shapes that match what the backend returns?

### Output format

Provide:

1. **Schema-to-API mapping table:**

   | Table | Model | DB Functions | Handlers | Routes | OpenAPI | Status |
   | ----- | ----- | ------------ | -------- | ------ | ------- | ------ |

2. **Frontend API consumption table:**

   | Frontend Action | HTTP Method | Endpoint | Backend Status | Notes |
   | --------------- | ----------- | -------- | -------------- | ----- |

3. **Stub endpoints** — List of `NotImplemented` handlers with what they need to become functional
4. **Missing endpoints** — New endpoints to add (with suggested path, method, and handler name)
5. **Frontend integration gaps** — Endpoints the frontend will need as new UI features are built (e.g., order management, team views)
6. **Implementation plan** — Prioritized order for completing the API, considering both foreign key dependencies and frontend needs

### Scope

Read all files in `migrations/` (V1 initial schema through V9, and any newer migrations), `src/models.rs`, `src/db/`, `src/handlers/`, `src/routes.rs`, `src/middleware/openapi.rs`, and `frontend/src/app.rs`. Do NOT modify any files — this is analysis only.
