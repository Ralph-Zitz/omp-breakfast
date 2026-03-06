# RBAC Rules Audit

Audit the codebase against the RBAC (Role-Based Access Control) policy defined below. For each rule, verify:

1. The handler enforces the correct role check (e.g., `require_admin`, `require_team_admin`, `require_team_member`, or self-only via `claims.sub`)
2. The corresponding OpenAPI annotation includes a `403 Forbidden` response
3. Integration tests exist covering both the allowed and denied cases
4. The error message returned matches the expected role requirement

## RBAC Policy Table

| Resource      | Action                        | Required Role                              | Enforced By                             |
| ------------- | ----------------------------- | ------------------------------------------ | --------------------------------------- |
| Team          | Create, Update, Delete        | Admin (global)                             | `require_admin`                         |
| Team Orders   | Delete All                    | Team Admin or Admin (global)               | `require_team_admin`                    |
| Team Orders   | Create                        | Team Member or above                       | `require_team_member`                   |
| Team Orders   | Update, Delete Single         | Order owner, Team Admin, or Admin (global) | `require_order_owner_or_team_admin`     |
| Team Members  | Add, Remove, Update Role      | Team Admin or Admin (global)               | `require_team_admin`                    |
| Team Members  | Assign Admin role             | Admin (global) only                        | `guard_admin_role_assignment`           |
| Team Members  | Demote/remove a global Admin  | Admin (global) only                        | `guard_admin_demotion`                  |
| Team Members  | Demote/remove last Admin      | Blocked for all                            | `guard_last_admin_membership`           |
| User          | Create                        | Admin or Team Admin (any team)             | `require_admin_or_team_admin`           |
| User          | Update, Delete (by ID/email)  | Self, Admin, or Team Admin (shared team)   | `require_self_or_admin_or_team_admin`   |
| Items         | Create, Update, Delete        | Admin (global)                             | `require_admin`                         |
| Roles         | Create, Update, Delete        | Admin (global)                             | `require_admin`                         |
| All read-only | GET endpoints                 | Any authenticated user                     | JWT auth middleware                     |

## Role Definitions

- **Admin (global):** User holds the "Admin" role in at least one team (checked via `db::is_admin` → `memberof` + `roles` table). Acts as superuser: bypasses all team-scoped and self-only checks. Can operate on any team without being a member.
- **Team Admin:** User holds the "Team Admin" role for the specific team being acted upon (checked via `db::get_member_role`). Can fully manage their team (members, orders, settings) but has no cross-team or system-wide powers. For user mutations (update, delete), a Team Admin can only act on users who are members of a team they administer (checked via `db::is_team_admin_of_user` — a self-join on `memberof`).
- **Team Member:** User holds any role (Team Admin, Member, or Guest) for the specific team (checked via `db::get_member_role`)
- **Self only:** The JWT `sub` claim must match the target user's ID. Admin (global) and Team Admin (of a shared team) bypass this check.

## Admin Demotion / Promotion Rules

- **Only global Admins can promote someone to Admin** (`guard_admin_role_assignment`): A Team Admin may assign any role *except* Admin. Only a global Admin may grant Admin privileges.
- **Only global Admins can demote or remove a global Admin** (`guard_admin_demotion`): If the target user holds the Admin role in *any* team, a Team Admin cannot change their role or remove them from a team. Only another global Admin may modify a global Admin's membership.
- **Global Admins can demote other global Admins**: There is no restriction preventing one Admin from changing another Admin's role, as long as at least one Admin remains.
- **Last admin protection** (`guard_last_admin_membership`): No user (including a global Admin) may demote or remove the last remaining global Admin. The guard uses `db::would_admins_remain_without` to verify at least one Admin would remain after excluding the target membership. Returns 403 if the operation would leave zero admins.
- **Members and Guests cannot modify roles**: The `require_team_admin` guard ensures that only Team Admins and global Admins can add, remove, or change member roles. Regular Members and Guests have no access to these operations.

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
5. **Admin demotion protection** is enforced: Team Admins cannot demote or remove a global Admin (`guard_admin_demotion` called in `update_member_role` and `remove_team_member`)
6. **Last admin protection** is enforced: No user can demote or remove the last global Admin (`guard_last_admin_membership` called after `guard_admin_demotion` in `update_member_role` and `remove_team_member`)
7. **OpenAPI annotations** include `403` response for every guarded endpoint
8. **First-user bootstrap** in `register_first_user` seeds the four default roles via `seed_default_roles()` and assigns the first user as Admin
9. **Role string constants** (`ROLE_ADMIN`, `ROLE_TEAM_ADMIN` in `middleware/auth.rs`) are used consistently across `db/membership.rs`, `handlers/mod.rs`, and `db/roles.rs` (`seed_default_roles`)

## Report Format

For each violation found, report:

- Which rule is violated
- Which file and handler
- What the expected enforcement is
- Severity: critical / important / minor
