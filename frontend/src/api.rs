use base64::Engine;
use gloo_net::http::{Request, RequestBuilder};
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::JsCast;

// ── API response types ──────────────────────────────────────────────────────

/// Health endpoint response.
#[derive(Clone, Debug, Deserialize)]
pub struct HealthResponse {
    #[allow(dead_code)]
    pub up: bool,
    pub setup_required: bool,
}

/// Paginated response envelope returned by all list endpoints.
#[derive(Clone, Debug, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    #[allow(dead_code)]
    pub total: i64,
    #[allow(dead_code)]
    pub limit: i64,
    #[allow(dead_code)]
    pub offset: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    #[allow(dead_code)]
    pub token_type: String,
    #[allow(dead_code)]
    pub expires_in: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JwtPayload {
    pub sub: String,
    #[serde(default)]
    pub exp: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserEntry {
    pub user_id: String,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub avatar_id: Option<String>,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AvatarListEntry {
    pub avatar_id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TeamEntry {
    pub team_id: String,
    pub tname: String,
    pub descr: Option<String>,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoleEntry {
    pub role_id: String,
    pub title: String,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ItemEntry {
    pub item_id: String,
    pub descr: String,
    pub price: String,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TeamOrderEntry {
    pub teamorders_id: String,
    pub teamorders_team_id: String,
    pub teamorders_user_id: String,
    pub pickup_user_id: Option<String>,
    pub duedate: Option<String>,
    pub closed: bool,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrderItemEntry {
    pub orders_teamorders_id: String,
    pub orders_item_id: String,
    pub orders_team_id: String,
    pub amt: i32,
    #[allow(dead_code)]
    pub created: String,
    #[allow(dead_code)]
    pub changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserInTeams {
    pub team_id: String,
    pub tname: String,
    pub descr: Option<String>,
    pub title: String,
    #[allow(dead_code)]
    pub firstname: String,
    #[allow(dead_code)]
    pub lastname: String,
    #[allow(dead_code)]
    pub joined: String,
    #[allow(dead_code)]
    pub role_changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UsersInTeam {
    pub user_id: String,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub title: String,
    #[allow(dead_code)]
    pub joined: String,
    #[allow(dead_code)]
    pub role_changed: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeletedResponse {
    #[allow(dead_code)]
    pub deleted: bool,
}

// ── User context (shared across all pages via provide_context) ──────────────

#[derive(Clone, Debug)]
pub struct UserContext {
    pub user_id: String,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub avatar_id: Option<String>,
    pub is_admin: bool,
    pub teams: Vec<UserInTeams>,
}

impl UserContext {
    /// Build a UserContext from a fetched UserEntry and team list.
    pub fn from_entry(user: UserEntry, teams: Vec<UserInTeams>) -> Self {
        let is_admin = teams.iter().any(|t| t.title == "Admin");
        Self {
            user_id: user.user_id,
            firstname: user.firstname,
            lastname: user.lastname,
            email: user.email,
            avatar_id: user.avatar_id,
            is_admin,
            teams,
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.firstname, self.lastname)
    }

    pub fn initials(&self) -> String {
        self.display_name()
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }
}

// ── JWT decode ──────────────────────────────────────────────────────────────

pub fn decode_jwt_payload(token: &str) -> Option<JwtPayload> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload).ok()
}

// ── Storage helpers ─────────────────────────────────────────────────────────

pub fn session_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.session_storage().ok())
        .flatten()
}

/// WARNING: Do NOT use `local_storage()` for storing authentication tokens.
/// Tokens are stored in `sessionStorage` (via [`session_storage()`]) to limit
/// exposure — they are cleared when the browser tab closes, reducing the
/// window for XSS-based token theft. `localStorage` persists across tabs and
/// sessions, making stolen tokens usable indefinitely. This helper exists
/// only for non-sensitive preferences (e.g., theme choice).
pub fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
}

// ── Token helpers ───────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

pub fn token_needs_refresh(token: &str, margin_secs: u64) -> bool {
    match decode_jwt_payload(token) {
        Some(payload) => match payload.exp {
            Some(exp) => now_secs() + margin_secs >= exp,
            None => false,
        },
        None => true,
    }
}

/// Attempt to refresh the access token using the stored refresh token.
/// Sends the old access token in the request body so the server can revoke it
/// immediately, closing the window where a leaked token could be reused.
pub async fn try_refresh_token() -> Option<String> {
    let storage = session_storage()?;
    let refresh_token = storage.get_item("refresh_token").ok().flatten()?;

    if refresh_token.is_empty() {
        return None;
    }

    // Capture the old access token before it's overwritten
    let old_access = storage.get_item("access_token").ok().flatten();

    let req = Request::post("/auth/refresh")
        .header("Authorization", &format!("Bearer {}", refresh_token));

    // Include the old access token so the server can revoke it
    let resp = if let Some(ref token) = old_access {
        #[derive(Serialize)]
        struct RefreshBody<'a> {
            access_token: &'a str,
        }
        req.json(&RefreshBody {
            access_token: token,
        })
        .ok()?
        .send()
        .await
        .ok()?
    } else {
        req.send().await.ok()?
    };

    if !resp.ok() {
        let _ = storage.remove_item("access_token");
        let _ = storage.remove_item("refresh_token");
        return None;
    }

    let auth: AuthResponse = resp.json().await.ok()?;
    let _ = storage.set_item("access_token", &auth.access_token);
    let _ = storage.set_item("refresh_token", &auth.refresh_token);
    Some(auth.access_token)
}

/// Get a valid access token, refreshing if expired or about to expire.
async fn get_valid_token() -> Option<String> {
    let token = session_storage()
        .and_then(|s| s.get_item("access_token").ok())
        .flatten()?;

    if token.is_empty() {
        return None;
    }

    if token_needs_refresh(&token, 60) {
        return try_refresh_token().await;
    }

    Some(token)
}

// ── Health check ────────────────────────────────────────────────────────────

/// Check the `/health` endpoint to determine if first-user setup is required.
pub async fn check_setup_required() -> bool {
    match Request::get("/health").send().await {
        Ok(resp) if resp.ok() => resp
            .json::<HealthResponse>()
            .await
            .map(|h| h.setup_required)
            .unwrap_or(false),
        _ => false,
    }
}

// ── HTTP helpers ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

fn build_method_request(method: HttpMethod, url: &str) -> RequestBuilder {
    match method {
        HttpMethod::Get => Request::get(url),
        HttpMethod::Post => Request::post(url),
        HttpMethod::Put => Request::put(url),
        HttpMethod::Delete => Request::delete(url),
    }
}

/// Perform an authenticated HTTP request with automatic token refresh.
pub async fn authed_request(
    method: HttpMethod,
    url: &str,
    body: Option<&serde_json::Value>,
) -> Option<gloo_net::http::Response> {
    let token = match get_valid_token().await {
        Some(t) => t,
        None => return None,
    };

    let send_once = |tok: String, m: HttpMethod, u: String, b: Option<serde_json::Value>| async move {
        let req = build_method_request(m, &u).header("Authorization", &format!("Bearer {}", tok));
        let result = match b.as_ref() {
            Some(v) => req.json(v).ok()?.send().await,
            None => req.send().await,
        };
        match result {
            Ok(r) => Some(r),
            Err(e) => {
                web_sys::console::warn_1(&format!("network error: {e}").into());
                None
            }
        }
    };

    let body_owned = body.cloned();
    let resp = send_once(token, method, url.to_string(), body_owned.clone()).await?;

    if resp.status() == 401 {
        if let Some(new_token) = try_refresh_token().await {
            return send_once(new_token, method, url.to_string(), body_owned).await;
        }
        return None;
    }

    Some(resp)
}

/// Convenience wrapper for authenticated GET requests.
pub async fn authed_get(url: &str) -> Option<gloo_net::http::Response> {
    authed_request(HttpMethod::Get, url, None).await
}

// ── Token revocation ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TokenRequest {
    token: String,
}

/// Revoke a token server-side. Fire-and-forget.
pub async fn revoke_token_server_side(bearer: &str, token_to_revoke: &str) {
    let req = Request::post("/auth/revoke")
        .header("Authorization", &format!("Bearer {}", bearer))
        .header("Content-Type", "application/json")
        .json(&TokenRequest {
            token: token_to_revoke.to_string(),
        });

    if let Ok(req) = req {
        let _ = req.send().await;
    }
}

// ── Data fetch helpers ──────────────────────────────────────────────────────

/// Fetch user details. Returns (user_id, display_name, email).
pub async fn fetch_user_details(
    access_token: &str,
) -> Option<(String, String, String, String, Option<String>, String)> {
    let payload = decode_jwt_payload(access_token)?;
    let url = format!("/api/v1.0/users/{}", payload.sub);
    let resp = authed_get(&url).await?;
    if !resp.ok() {
        web_sys::console::warn_1(
            &format!(
                "fetch_user_details: non-OK response (status {})",
                resp.status()
            )
            .into(),
        );
        return None;
    }
    let user: UserEntry = resp.json().await.ok()?;
    let display = format!("{} {}", user.firstname, user.lastname);
    Some((
        payload.sub,
        user.firstname,
        user.lastname,
        user.email,
        user.avatar_id,
        display,
    ))
}

/// Fetch the user's team memberships to determine roles.
pub async fn fetch_user_teams(user_id: &str) -> Vec<UserInTeams> {
    let url = format!("/api/v1.0/users/{}/teams", user_id);
    match authed_get(&url).await {
        Some(resp) if resp.ok() => resp
            .json::<PaginatedResponse<UserInTeams>>()
            .await
            .map(|p| p.items)
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Build a full UserContext after login or session restore.
pub async fn build_user_context(access_token: &str) -> Option<UserContext> {
    let (user_id, firstname, lastname, email, avatar_id, _display) =
        fetch_user_details(access_token).await?;
    let teams = fetch_user_teams(&user_id).await;
    let entry = UserEntry {
        user_id,
        firstname,
        lastname,
        email,
        avatar_id,
        created: String::new(),
        changed: String::new(),
    };
    Some(UserContext::from_entry(entry, teams))
}

// ── Async sleep (for toast auto-dismiss) ────────────────────────────────────

pub async fn sleep_ms(ms: i32) {
    use wasm_bindgen::closure::Closure;

    let promise = js_sys::Promise::new(&mut move |resolve, _reject| {
        let cb = Closure::once_into_js(move || {
            let _ = resolve.call0(&wasm_bindgen::JsValue::UNDEFINED);
        });
        web_sys::window()
            .expect("no global window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms)
            .expect("set_timeout failed");
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
