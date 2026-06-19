use std::num::NonZeroU32;

use dioxus::fullstack::Lazy;
use sqlx::pool::PoolConnection;
use sqlx::{Row, Sqlite, SqlitePool};

use crate::server::{config::CONFIG, gen_token, kanidm::Kanidm, now};
use crate::shared::{
    CreateInviteReq, CurrentUser, InvitationView, InviteStatus, InviteePrompt, SignupForm,
    Unavailable, ValidUsername,
};

/// Lazily-initialised connection pool. First access connects and runs the
/// schema migration, blocking until ready.
pub static DB: Lazy<SqlitePool> = Lazy::new(|| async {
    let pool = SqlitePool::connect(&CONFIG.database_url).await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS invitations (
            token            TEXT PRIMARY KEY,
            label            TEXT NOT NULL,
            created_by       TEXT NOT NULL,
            created_at       INTEGER NOT NULL,
            expires_at       INTEGER NOT NULL,
            max_uses         INTEGER,
            accounts_created INTEGER NOT NULL DEFAULT 0,
            revoked          INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(&pool)
    .await?;
    dioxus::Ok(pool)
});

fn read_max_uses(raw: Option<i64>) -> Option<NonZeroU32> {
    raw.and_then(|v| u32::try_from(v).ok()).and_then(NonZeroU32::new)
}

fn read_count(raw: i64) -> u32 {
    u32::try_from(raw).unwrap_or(0)
}

/// Compute lifecycle status; precedence is revoked > expired > exhausted.
fn status(
    now: i64,
    expires_at: i64,
    revoked: bool,
    max_uses: Option<NonZeroU32>,
    used: u32,
) -> InviteStatus {
    if revoked {
        InviteStatus::Revoked
    } else if expires_at <= now {
        InviteStatus::Expired
    } else if max_uses.is_some_and(|m| used >= m.get()) {
        InviteStatus::Exhausted
    } else {
        InviteStatus::Active
    }
}

fn row_to_view(r: &sqlx::sqlite::SqliteRow, viewer: &CurrentUser, now: i64) -> InvitationView {
    let created_by: String = r.get("created_by");
    let expires_at: i64 = r.get("expires_at");
    let revoked = r.get::<i64, _>("revoked") != 0;
    let max_uses = read_max_uses(r.get("max_uses"));
    let accounts_created = read_count(r.get("accounts_created"));
    InvitationView {
        token: r.get("token"),
        label: r.get("label"),
        created_at: r.get("created_at"),
        expires_at,
        max_uses,
        accounts_created,
        status: status(now, expires_at, revoked, max_uses, accounts_created),
        owned: created_by == viewer.sub,
    }
}

/// Invitations the viewer may see: their own, or all for an admin.
pub async fn list_invitations(viewer: &CurrentUser) -> anyhow::Result<Vec<InvitationView>> {
    let now = now();
    let rows = if viewer.role.is_admin() {
        sqlx::query(
            "SELECT token,label,created_by,created_at,expires_at,max_uses,accounts_created,revoked
             FROM invitations ORDER BY created_at DESC",
        )
        .fetch_all(&*DB)
        .await?
    } else {
        sqlx::query(
            "SELECT token,label,created_by,created_at,expires_at,max_uses,accounts_created,revoked
             FROM invitations WHERE created_by = ?1 ORDER BY created_at DESC",
        )
        .bind(&viewer.sub)
        .fetch_all(&*DB)
        .await?
    };
    Ok(rows.iter().map(|r| row_to_view(r, viewer, now)).collect())
}

pub async fn create_invitation(
    owner: &CurrentUser,
    req: CreateInviteReq,
) -> anyhow::Result<InvitationView> {
    let token = gen_token();
    let now = now();
    let expires_at = now + i64::from(req.ttl.seconds());
    let max_db: Option<i64> = req.max_uses.map(|m| i64::from(m.get()));
    sqlx::query(
        "INSERT INTO invitations
            (token,label,created_by,created_at,expires_at,max_uses,accounts_created,revoked)
         VALUES (?1,?2,?3,?4,?5,?6,0,0)",
    )
    .bind(&token)
    .bind(&req.label)
    .bind(&owner.sub)
    .bind(now)
    .bind(expires_at)
    .bind(max_db)
    .execute(&*DB)
    .await?;
    Ok(InvitationView {
        token,
        label: req.label,
        created_at: now,
        expires_at,
        max_uses: req.max_uses,
        accounts_created: 0,
        status: InviteStatus::Active,
        owned: true,
    })
}

/// Revoke an invitation. Non-admins may only revoke their own.
pub async fn revoke(token: &str, actor: &CurrentUser) -> anyhow::Result<()> {
    let result = if actor.role.is_admin() {
        sqlx::query("UPDATE invitations SET revoked = 1 WHERE token = ?1")
            .bind(token)
            .execute(&*DB)
            .await?
    } else {
        sqlx::query("UPDATE invitations SET revoked = 1 WHERE token = ?1 AND created_by = ?2")
            .bind(token)
            .bind(&actor.sub)
            .execute(&*DB)
            .await?
    };
    if result.rows_affected() == 0 {
        anyhow::bail!("invitation not found or not yours");
    }
    Ok(())
}

/// Public view of an invitation for an invitee opening the link.
pub async fn prompt(token: &str) -> anyhow::Result<InviteePrompt> {
    let now = now();
    let row = sqlx::query("SELECT label,expires_at,max_uses,accounts_created,revoked FROM invitations WHERE token = ?1")
        .bind(token)
        .fetch_optional(&*DB)
        .await?;
    let Some(r) = row else {
        return Ok(InviteePrompt::Unavailable(Unavailable::NotFound));
    };
    let label: String = r.get("label");
    let expires_at: i64 = r.get("expires_at");
    let revoked = r.get::<i64, _>("revoked") != 0;
    let max_uses = read_max_uses(r.get("max_uses"));
    let used = read_count(r.get("accounts_created"));
    Ok(match status(now, expires_at, revoked, max_uses, used) {
        InviteStatus::Active => InviteePrompt::Open { label },
        InviteStatus::Expired => InviteePrompt::Unavailable(Unavailable::Expired),
        InviteStatus::Revoked => InviteePrompt::Unavailable(Unavailable::Revoked),
        InviteStatus::Exhausted => InviteePrompt::Unavailable(Unavailable::Exhausted),
    })
}

/// Redeem an invitation: validate, provision the kanidm person, mint the reset
/// token, and bump the counter — all under a held write lock so the max-uses
/// cap is exact. Returns the kanidm reset URL to redirect the invitee to.
pub async fn redeem(token: &str, form: SignupForm) -> anyhow::Result<String> {
    // Parse external input into strong types before touching the database.
    let username = ValidUsername::parse(&form.username).map_err(|e| anyhow::anyhow!(e))?;
    let displayname = form.displayname.trim();
    if displayname.is_empty() {
        anyhow::bail!("display name is required");
    }
    let email = form.email.trim();
    if email.is_empty() {
        anyhow::bail!("email is required");
    }

    let mut conn = DB.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
    let result = redeem_locked(&mut conn, token, &username, displayname, email).await;
    match &result {
        Ok(_) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
        }
        Err(_) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        }
    }
    result
}

async fn redeem_locked(
    conn: &mut PoolConnection<Sqlite>,
    token: &str,
    username: &ValidUsername,
    displayname: &str,
    email: &str,
) -> anyhow::Result<String> {
    let now = now();
    let row =
        sqlx::query("SELECT expires_at,max_uses,accounts_created,revoked FROM invitations WHERE token = ?1")
            .bind(token)
            .fetch_optional(&mut **conn)
            .await?;
    let Some(r) = row else {
        anyhow::bail!("invitation not found");
    };
    let expires_at: i64 = r.get("expires_at");
    let revoked = r.get::<i64, _>("revoked") != 0;
    let max_uses = read_max_uses(r.get("max_uses"));
    let used = read_count(r.get("accounts_created"));
    match status(now, expires_at, revoked, max_uses, used) {
        InviteStatus::Active => {}
        InviteStatus::Expired => anyhow::bail!("invitation expired"),
        InviteStatus::Revoked => anyhow::bail!("invitation revoked"),
        InviteStatus::Exhausted => anyhow::bail!("invitation fully used"),
    }

    // Provision the account and mint the reset token while still holding the
    // lock. kanidm owns uniqueness, so a duplicate username surfaces here and
    // rolls the transaction back without consuming a use.
    let kanidm = Kanidm::new()?;
    kanidm
        .create_person(username.as_str(), displayname, email)
        .await?;
    let intent = kanidm.credential_update_intent(username.as_str()).await?;

    sqlx::query("UPDATE invitations SET accounts_created = accounts_created + 1 WHERE token = ?1")
        .bind(token)
        .execute(&mut **conn)
        .await?;

    Ok(kanidm.reset_url(&intent))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> Option<NonZeroU32> {
        NonZeroU32::new(n)
    }

    #[test]
    fn active_when_unexpired_unrevoked_and_under_cap() {
        assert_eq!(status(100, 200, false, nz(5), 2), InviteStatus::Active);
    }

    #[test]
    fn no_cap_is_never_exhausted() {
        assert_eq!(status(100, 200, false, None, 9_999), InviteStatus::Active);
    }

    #[test]
    fn exhausted_at_or_above_cap() {
        assert_eq!(status(100, 200, false, nz(3), 3), InviteStatus::Exhausted);
        assert_eq!(status(100, 200, false, nz(3), 4), InviteStatus::Exhausted);
    }

    #[test]
    fn expired_when_now_reaches_expiry() {
        assert_eq!(status(200, 200, false, None, 0), InviteStatus::Expired);
        assert_eq!(status(201, 200, false, None, 0), InviteStatus::Expired);
    }

    #[test]
    fn revoked_takes_precedence_over_expired_and_exhausted() {
        assert_eq!(status(300, 200, true, nz(1), 5), InviteStatus::Revoked);
    }

    #[test]
    fn expired_takes_precedence_over_exhausted() {
        assert_eq!(status(300, 200, false, nz(1), 5), InviteStatus::Expired);
    }

    #[test]
    fn read_max_uses_drops_zero_and_negative() {
        assert_eq!(read_max_uses(None), None);
        assert_eq!(read_max_uses(Some(0)), None);
        assert_eq!(read_max_uses(Some(-5)), None);
        assert_eq!(read_max_uses(Some(7)), nz(7));
    }

    #[test]
    fn read_count_clamps_negative_to_zero() {
        assert_eq!(read_count(-1), 0);
        assert_eq!(read_count(42), 42);
    }
}
