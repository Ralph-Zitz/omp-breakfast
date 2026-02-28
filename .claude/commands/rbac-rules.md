# RBAC Rules Audit

Audit the codebase against the RBAC (Role-Based Access Control) policy defined below. For each rule, verify:

1. The handler enforces the correct role check (e.g., `require_admin`, `require_team_admin`, `require_team_member`, or self-only via `claims.sub`)
2. The corresponding OpenAPI annotation includes a `403 Forbidden` response
3. Integration tests exist covering both the allowed and denied cases
4. The error message returned matches the expected role requirement

## RBAC Policy Table

| Resource       | Action                        | Required Role          | Enforced By           |
| -------------- | ----------------------------- | ---------------------- | --------------------- |
| Team           | Create, Update, Delete        | Admin (global)         | `require_admin`       |
| Team Orders    | Delete All                    | Team Admin             | `require_team_admin`  |
| Team Orders    | Create, Update, Delete Single | Team Member            | `require_team_member` |
| Team Members   | Add, Remove, Update Role      | Team Admin             | `require_team_admin`  |
| User           | Update, Delete (by ID/email)  | Self only              | `claims.sub` check    |
| Items          | Create, Update, Delete        | Any authenticated user | JWT auth middleware   |
| Roles          | Create, Update, Delete        | Any authenticated user | JWT auth middleware   |
| All read-only  | GET endpoints                 | Any authenticated user | JWT auth middleware   |

## Role Definitions

- **Admin (global):** User holds the "Admin" role in at least one team (checked via `db::is_admin` → `memberof` + `roles` table)
- **Team Admin:** User holds the "Admin" role for the specific team being acted upon (checked via `db::get_member_role`)
- **Team Member:** User holds any role for the specific team (checked via `db::get_member_role`)
- **Self only:** The JWT `sub` claim must match the target user's ID

## Report Format

For each violation found, report:

- Which rule is violated
- Which file and handler
- What the expected enforcement is
- Severity: critical / important / minor
