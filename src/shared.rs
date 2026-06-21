//! Types shared between the client (wasm) and the server. Must only depend on
//! `serde` so they compile on both targets.
//!
//! Design rule: illegal states are unrepresentable. Sum types replace
//! flag-plus-payload pairs, and constrained values are newtypes that can only
//! be built through a validating constructor (enforced even across the wire via
//! custom `Deserialize`).
//!
//! These types are the cross-stack contract: some are consumed only by the
//! server (e.g. `ValidUsername`, `SignupForm`), so on a client-only build they
//! read as dead code even though the full app uses them.
#![cfg_attr(not(feature = "server"), allow(dead_code))]

use std::num::NonZeroU32;

use serde::{Deserialize, Deserializer, Serialize};

/// An authenticated user is always an inviter; admins are a strict superset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Inviter,
    Admin,
}

impl Role {
    pub fn is_admin(self) -> bool {
        matches!(self, Role::Admin)
    }
}

/// The signed-in inviter, as surfaced to the UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    /// Stable kanidm subject id; used for invitation ownership.
    pub sub: String,
    pub username: String,
    pub display_name: String,
    pub role: Role,
}

/// Lifecycle status of an invitation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteStatus {
    Active,
    Expired,
    Revoked,
    Exhausted,
}

impl InviteStatus {
    pub fn label(self) -> &'static str {
        match self {
            InviteStatus::Active => "Active",
            InviteStatus::Expired => "Expired",
            InviteStatus::Revoked => "Revoked",
            InviteStatus::Exhausted => "Fully used",
        }
    }
}

/// A single invitation as shown on the dashboard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InvitationView {
    pub token: String,
    pub label: String,
    pub created_at: i64,
    /// Absolute expiry in unix seconds; `None` for a non-expiring invitation.
    pub expires_at: Option<i64>,
    pub max_uses: Option<NonZeroU32>,
    pub accounts_created: u32,
    pub status: InviteStatus,
    /// Whether the viewing inviter owns this invitation.
    pub owned: bool,
}

/// Payload for the home page.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dashboard {
    pub user: Option<CurrentUser>,
    pub invites: Vec<InvitationView>,
}

/// Why an invitation cannot currently be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unavailable {
    NotFound,
    Expired,
    Revoked,
    Exhausted,
}

impl Unavailable {
    pub fn message(self) -> &'static str {
        match self {
            Unavailable::NotFound => "This invitation link is not valid.",
            Unavailable::Expired => "This invitation has expired.",
            Unavailable::Revoked => "This invitation has been revoked.",
            Unavailable::Exhausted => "This invitation has already been fully used.",
        }
    }
}

/// What an invitee sees when opening a link.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InviteePrompt {
    Open,
    Unavailable(Unavailable),
}

/// A validated invitation time-to-live in seconds. Construction is bounded and
/// enforced on deserialization, so an out-of-range TTL can never exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Ttl(u32);

impl Ttl {
    pub const MIN: u32 = 60;
    pub const MAX: u32 = 30 * 24 * 3600;

    pub fn new(secs: u32) -> Option<Ttl> {
        (Self::MIN..=Self::MAX).contains(&secs).then_some(Ttl(secs))
    }

    pub fn seconds(self) -> u32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Ttl {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let secs = u32::deserialize(d)?;
        Ttl::new(secs).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "ttl must be between {} and {} seconds",
                Ttl::MIN,
                Ttl::MAX
            ))
        })
    }
}

/// A kanidm `name` that has passed format validation. Cannot be constructed
/// from an invalid string. kanidm remains the authority on uniqueness.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ValidUsername(String);

impl ValidUsername {
    pub fn parse(name: &str) -> Result<ValidUsername, String> {
        if name.is_empty() {
            return Err("Username is required".into());
        }
        if name.len() > 64 {
            return Err("Username must be 64 characters or fewer".into());
        }
        let first = name.chars().next().unwrap();
        if !first.is_ascii_lowercase() {
            return Err("Username must start with a lowercase letter".into());
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
        {
            return Err(
                "Username may only contain lowercase letters, digits, '.', '-' and '_'".into(),
            );
        }
        Ok(ValidUsername(name.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Request body for creating an invitation. `ttl` is `None` for a non-expiring
/// link; `max_uses` is `None` for unlimited.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateInviteReq {
    pub label: String,
    pub ttl: Option<Ttl>,
    pub max_uses: Option<NonZeroU32>,
}

/// The invitee's submitted account details — raw external input from an HTML
/// form. Parsed into stronger types on the server before use.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignupForm {
    pub username: String,
    pub displayname: String,
    pub email: String,
}

/// The result of a signup attempt, as data rather than a transport error: the
/// form needs to tell these cases apart to show the right message. Only genuine
/// network/RPC failures surface as an `Err` from the server function.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SignupOutcome {
    /// Account created; send the invitee to this kanidm credential-reset URL.
    Success { reset_url: String },
    /// The chosen username is already taken — the common, correctable case.
    UsernameTaken,
    /// The submitted details were rejected, with a reason to show.
    Invalid(String),
    /// The invitation is no longer usable (expired, revoked, exhausted, gone).
    Unavailable,
    /// An unexpected server-side failure.
    Internal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_username_accepts_typical_names() {
        assert!(ValidUsername::parse("alice").is_ok());
        assert!(ValidUsername::parse("a").is_ok());
        assert!(ValidUsername::parse("a.b-c_1").is_ok());
        assert_eq!(ValidUsername::parse("alice").unwrap().as_str(), "alice");
    }

    #[test]
    fn valid_username_rejects_empty() {
        assert!(ValidUsername::parse("").is_err());
    }

    #[test]
    fn valid_username_enforces_length_at_the_boundary() {
        assert!(ValidUsername::parse(&"a".repeat(64)).is_ok());
        assert!(ValidUsername::parse(&"a".repeat(65)).is_err());
    }

    #[test]
    fn valid_username_must_start_with_lowercase_letter() {
        assert!(ValidUsername::parse("1abc").is_err());
        assert!(ValidUsername::parse(".abc").is_err());
        assert!(ValidUsername::parse("Abc").is_err());
    }

    #[test]
    fn valid_username_rejects_disallowed_characters() {
        assert!(ValidUsername::parse("ab c").is_err());
        assert!(ValidUsername::parse("ab@c").is_err());
        assert!(ValidUsername::parse("abC").is_err());
    }

    #[test]
    fn ttl_new_enforces_inclusive_bounds() {
        assert!(Ttl::new(Ttl::MIN - 1).is_none());
        assert!(Ttl::new(Ttl::MIN).is_some());
        assert!(Ttl::new(Ttl::MAX).is_some());
        assert!(Ttl::new(Ttl::MAX + 1).is_none());
        assert_eq!(Ttl::new(3600).unwrap().seconds(), 3600);
    }

    // The "enforced across the wire" guarantee depends on the custom
    // Deserialize. serde_json is only available on the server build.
    #[cfg(feature = "server")]
    #[test]
    fn ttl_deserialize_rejects_out_of_range() {
        assert!(serde_json::from_str::<Ttl>(&Ttl::MIN.to_string()).is_ok());
        assert!(serde_json::from_str::<Ttl>(&Ttl::MAX.to_string()).is_ok());
        assert!(serde_json::from_str::<Ttl>(&(Ttl::MIN - 1).to_string()).is_err());
        assert!(serde_json::from_str::<Ttl>(&(Ttl::MAX + 1).to_string()).is_err());
    }
}
