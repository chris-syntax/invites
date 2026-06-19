use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use dioxus::fullstack::HeaderMap;

use crate::shared::CurrentUser;

/// Session cookie name.
pub const COOKIE: &str = "invites_session";

/// In-memory session store. Adequate for a single-instance internal tool;
/// sessions are dropped on restart (users simply sign in again).
static SESSIONS: LazyLock<Mutex<HashMap<String, CurrentUser>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Resolve the signed-in user from request headers, if any.
pub fn current_user(headers: &HeaderMap) -> Option<CurrentUser> {
    let sid = session_id(headers)?;
    SESSIONS.lock().ok()?.get(&sid).cloned()
}

/// Extract the session id from the `Cookie` header.
pub fn session_id(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("cookie")?.to_str().ok()?;
    let prefix = format!("{COOKIE}=");
    raw.split(';')
        .find_map(|part| part.trim().strip_prefix(&prefix).map(str::to_string))
}

/// Create a new session and return its id (to be set as a cookie).
pub fn create(user: CurrentUser) -> String {
    let sid = crate::server::gen_token();
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.insert(sid.clone(), user);
    }
    sid
}

/// Drop a session.
pub fn destroy(sid: &str) {
    if let Ok(mut sessions) = SESSIONS.lock() {
        sessions.remove(sid);
    }
}
