# New UI Components

Custom frontend components created for OMP Breakfast that are not provided by the CONNECT Design System. Each component follows the CONNECT naming conventions (`--ds-*` tokens, `.connect-*` class patterns) and is documented here per the project convention.

## Sidebar

- **File:** `frontend/src/components/sidebar.rs`
- **Purpose:** Fixed left-side navigation panel with brand header, navigation items, theme toggle, user info, and logout button. Collapses on mobile with hamburger menu overlay.
- **Props:** None (reads `Page`, `UserContext`, and `sidebar_open` signals from Leptos context)
- **Sub-components:**
  - `MobileHeader` — Top bar with hamburger button and brand text, visible only on viewports ≤ 768px
  - `NavItem` — Individual navigation link with icon + label; highlights when active
  - `LogoutButton` — Revokes tokens server-side, clears session storage, navigates to Login
- **Rationale:** CONNECT provides `.connect-menu-item` for individual menu items but not a complete sidebar shell with brand, footer, responsive collapse, and overlay. The sidebar composes CONNECT menu items, avatars, dividers, and buttons.

## Card

- **File:** `frontend/src/components/card.rs`
- **Purpose:** General-purpose content container with CONNECT surface background, border, and shadow.
- **Props:**
  - `children: Children` — content to render inside the card
  - `extra_class: &'static str` (optional, default `""`) — additional CSS class(es) to append
- **Rationale:** CONNECT does not provide a generic card component. Uses DS tokens for background (`--ds-color-surface-background-level-1-default`), border (`--ds-color-stroke-subtle`), radius, and elevation.

## PageHeader

- **File:** `frontend/src/components/card.rs`
- **Purpose:** Page-level header with title and optional action buttons (e.g., "New Team" button).
- **Props:**
  - `title: &'static str` — the page heading text
  - `children: Children` (optional) — action buttons or other elements rendered to the right
- **Rationale:** CONNECT does not include a page-header component. Uses DS heading typography tokens.

## ThemeToggle

- **File:** `frontend/src/components/theme_toggle.rs`
- **Purpose:** Light/dark mode toggle switch. Reads OS preference on first load and stores the choice in `localStorage`. Applies theme by setting `data-mode` attribute on `<html>`.
- **Props:** None
- **Exported functions:**
  - `init_theme()` — call at app startup to apply stored or OS-preferred theme
  - `toggle_theme()` — switch between light and dark mode
  - `is_dark_mode() -> bool` — query current mode
- **Rationale:** CONNECT provides the `.connect-toggle` component CSS but not the toggle logic or theme persistence. The enterprise theme activates via `data-mode="dark"` on the root element, which this component manages.

## Toast

- **File:** `frontend/src/components/toast.rs`
- **Purpose:** Non-blocking notification system for success/error/warning/info messages. Auto-dismisses after 5 seconds.
- **Props (ToastRegion):** None (reads `ToastContext` from Leptos context)
- **Exported types:**
  - `ToastVariant` — `Success`, `Negative`, `Warning`, `Informative`
  - `ToastContext` — Context struct with `push(variant, message)` and `dismiss(index)` methods
- **Convenience functions:** `toast_success(msg)`, `toast_error(msg)`
- **Rationale:** CONNECT provides `.connect-toast` styling and icon patterns, but not the reactive state management, auto-dismiss timer, or positioning logic needed for a toast notification system.

## ConfirmModal

- **File:** `frontend/src/components/modal.rs`
- **Purpose:** Confirmation dialog for destructive actions (delete user, remove team member, etc.). Renders a backdrop overlay with a centered dialog.
- **Props:**
  - `open: ReadSignal<bool>` — controls visibility
  - `title: &'static str` — dialog heading
  - `message: String` — body text explaining the action
  - `confirm_label: &'static str` — text for the confirm button (e.g., "Delete")
  - `destructive: bool` — when true, confirm button uses `connect-button--negative` styling
  - `on_confirm: impl Fn() + 'static` — callback when user confirms
  - `on_cancel: impl Fn() + 'static` — callback when user cancels (also triggered by backdrop click)
- **Rationale:** CONNECT provides modal-adjacent styling but not the full dialog component with backdrop, focus management, and destructive-action patterns.

## Icon

- **File:** `frontend/src/components/icons.rs`
- **Purpose:** Inline SVG icon component using paths from the CONNECT icon library (`connect-icons/svg/`).
- **Props:**
  - `kind: IconKind` — which icon to render (18 variants available)
  - `size: u32` (optional, default `20`) — icon width and height in pixels
- **Available icons:** House, Users, User, ShieldCheck, Tag, ClipboardList, Bars, Sun, Moon, ArrowRightFromBracket, CirclePlus, PenToSquare, Trash, CircleCheck, CircleXmark, CircleInfo, TriangleExclamation, Gear
- **Rationale:** CONNECT ships icons as individual SVG files (`connect-icons/svg/*.svg`). Trunk cannot import raw SVG files at build time. This component embeds the SVG `d` path data directly from the design system source files, avoiding external HTTP requests and enabling inline coloring via `fill="currentColor"`.

## LoadingSpinner

- **File:** `frontend/src/components/mod.rs`
- **Purpose:** Indeterminate progress circle shown while data is loading. Used on all list pages (Teams, Items, Orders, Roles, Admin) as a shared loading state indicator.
- **Props:** None
- **Rationale:** CONNECT provides `.connect-progress-circle` CSS and the `connect-progress-circle--indeterminate` modifier, but not a Leptos component wrapper. This component wraps the CONNECT markup in a reusable `#[component]` so pages don't repeat the SVG structure.

## FormDialog

- **File:** `frontend/src/components/modal.rs`
- **Purpose:** Generic form dialog wrapping the modal overlay/dialog/header/body/footer boilerplate. Form fields are passed as `children` and rendered inside the modal body. Used by CRUD create/edit dialogs to avoid repeating ~40 lines of structural markup.
- **Props:**
  - `open: Signal<bool>` — controls visibility
  - `title: &'static str` — dialog heading
  - `submit_label: &'static str` (optional, default `"Create"`) — text for the submit button
  - `disabled: Signal<bool>` — when true, submit button is disabled (form validation)
  - `on_submit: impl Fn() + 'static` — callback when user clicks submit
  - `on_cancel: impl Fn() + 'static` — callback when user cancels (also triggered by backdrop click)
  - `children: Children` — form field markup rendered inside the modal body
- **Rationale:** CONNECT provides modal-adjacent styling but not a reusable form dialog shell. All CRUD pages (roles, items, teams, admin) use the same overlay→dialog→header→body→footer structure with Cancel/Submit buttons. This component extracts that shared structure so individual dialogs only define their form fields and validation logic.
