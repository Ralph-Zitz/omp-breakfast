# RBAC Rules Audit

Audit the codebase against the RBAC (Role-Based Access Control) policy defined below. For each rule, verify:

1. The handler enforces the correct role check (e.g., `require_admin`, `require_team_admin`, `require_team_member`, or self-only via `claims.sub`)
2. The corresponding OpenAPI annotation includes a `403 Forbidden` response
3. Integration tests exist covering both the allowed and denied cases
4. The error message returned matches the expected role requirement

## RBAC Policy Table

| Resource      | Action                        | Required Role                      | Enforced By                   |
| ------------- | ----------------------------- | ---------------------------------- | ----------------------------- |
| Team          | Create, Update, Delete        | Admin (global)                     | `require_admin`               |
| Team Orders   | Delete All                    | Team Admin or Admin (global)       | `require_team_admin`          |
| Team Orders   | Create, Update, Delete Single | Team Member or above               | `require_team_member`         |
| Team Members  | Add, Remove, Update Role      | Team Admin or Admin (global)       | `require_team_admin`          |
| User          | Create                        | Admin or Team Admin (any team)     | `require_admin_or_team_admin` |
| User          | Update, Delete (by ID/email)  | Self, Admin, or Team Admin (shared team) | `require_self_or_admin_or_team_admin` |
| Items         | Create, Update, Delete        | Admin (global)                     | `require_admin`               |
| Roles         | Create, Update, Delete        | Admin (global)                     | `require_admin`               |
| All read-only | GET endpoints                 | Any authenticated user             | JWT auth middleware            |

## Role Definitions

- **Admin (global):** User holds the "Admin" role in at least one team (checked via `db::is_admin` → `memberof` + `roles` table). Acts as superuser: bypasses all team-scoped and self-only checks. Can operate on any team without being a member.
- **Team Admin:** User holds the "Team Admin" role for the specific team being acted upon (checked via `db::get_member_role`). Can fully manage their team (members, orders, settings) but has no cross-team or system-wide powers. For user mutations (update, delete), a Team Admin can only act on users who are members of a team they administer (checked via `db::is_team_admin_of_user` — a self-join on `memberof`).
- **Team Member:** User holds any role (Team Admin, Member, or Guest) for the specific team (checked via `db::get_member_role`)
- **Self only:** The JWT `sub` claim must match the target user's ID. Admin (global) and Team Admin (of a shared team) bypass this check.

## Admin Bypass Rules

Global Admin must bypass the following checks:

- `require_team_admin`: Admin can manage any team's members and orders without being a Team Admin
- `require_team_member`: Admin can act on any team without being a member
- Self-only user mutations: Admin can update/delete any user, not just themselves

Team Admin bypasses the self-only check for user mutations **only when** the target user is a member of a team they administer (checked via `db::is_team_admin_of_user`). A Team Admin cannot modify users who are only in other teams or who have no team membership.

## Audit Checks

For each handler in `src/handlers/`, verify:

1. **Correct guard function** is called per the policy table above
2. **Admin bypass** works on all team-scoped and self-only endpoints
3. **Team Admin vs Admin distinction** is enforced: Team Admin cannot create/delete teams, CUD items, or CUD roles
4. **Team Admin user scoping** is enforced: Team Admin can only update/delete users who share a team they administer (via `db::is_team_admin_of_user`)
5. **OpenAPI annotations** include `403` response for every guarded endpoint
6. **Seed data** in `database_seed.sql` assigns roles correctly: "Admin" for global admins, "Team Admin" for team-scoped admins
7. **Role string constants** (`ROLE_ADMIN`, `ROLE_TEAM_ADMIN` in `middleware/auth.rs`) are used consistently across `db/membership.rs`, `handlers/mod.rs`, and `database_seed.sql` — no hardcoded role strings

## Report Format

For each violation found, report:

- Which rule is violated
- Which file and handler
- What the expected enforcement is
- Severity: critical / important / minor
