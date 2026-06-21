use std::num::NonZeroU32;

use dioxus::fullstack::Lazy;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, Database, DatabaseConnection,
    DatabaseTransaction, DbErr, EntityTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use sea_orm_migration::MigratorTrait;

use crate::server::migration::Migrator;
use crate::server::models::invitations::{
    self, ActiveModel, Column, Entity as Invitations,
};
use crate::server::kanidm::CreatePersonError;
use crate::server::{config::CONFIG, gen_token, kanidm::Kanidm, now};
use crate::shared::{
    CreateInviteReq, CurrentUser, InvitationView, InviteStatus, InviteePrompt, SignupForm,
    Unavailable, ValidUsername,
};

/// Lazily-initialised database connection. First access connects and runs any
/// outstanding migrations, blocking until ready.
pub static DB: Lazy<DatabaseConnection> = Lazy::new(|| async {
    let db = Database::connect(&CONFIG.database_url).await?;
    Migrator::up(&db, None).await?;
    dioxus::Ok(db)
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
    expires_at: Option<i64>,
    revoked: bool,
    max_uses: Option<NonZeroU32>,
    used: u32,
) -> InviteStatus {
    if revoked {
        InviteStatus::Revoked
    } else if expires_at.is_some_and(|e| e <= now) {
        InviteStatus::Expired
    } else if max_uses.is_some_and(|m| used >= m.get()) {
        InviteStatus::Exhausted
    } else {
        InviteStatus::Active
    }
}

fn model_to_view(m: &invitations::Model, viewer: &CurrentUser, now: i64) -> InvitationView {
    let max_uses = read_max_uses(m.max_uses);
    let accounts_created = read_count(m.accounts_created);
    InvitationView {
        token: m.token.clone(),
        label: m.label.clone(),
        created_at: m.created_at,
        expires_at: m.expires_at,
        max_uses,
        accounts_created,
        status: status(now, m.expires_at, m.revoked, max_uses, accounts_created),
        owned: m.created_by == viewer.sub,
    }
}

/// Invitations the viewer may see: their own, or all for an admin.
pub async fn list_invitations(viewer: &CurrentUser) -> anyhow::Result<Vec<InvitationView>> {
    let now = now();
    let mut query = Invitations::find().order_by_desc(Column::CreatedAt);
    if !viewer.role.is_admin() {
        query = query.filter(Column::CreatedBy.eq(&viewer.sub));
    }
    let rows = query.all(&*DB).await?;
    Ok(rows.iter().map(|m| model_to_view(m, viewer, now)).collect())
}

pub async fn create_invitation(
    owner: &CurrentUser,
    req: CreateInviteReq,
) -> anyhow::Result<InvitationView> {
    let token = gen_token();
    let now = now();
    let expires_at = req.ttl.map(|t| now + i64::from(t.seconds()));
    let max_db: Option<i64> = req.max_uses.map(|m| i64::from(m.get()));
    ActiveModel {
        token: Set(token.clone()),
        label: Set(req.label.clone()),
        created_by: Set(owner.sub.clone()),
        created_at: Set(now),
        expires_at: Set(expires_at),
        max_uses: Set(max_db),
        accounts_created: Set(0),
        revoked: Set(false),
    }
    .insert(&*DB)
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
    let mut update = Invitations::update_many()
        .col_expr(Column::Revoked, Expr::value(true))
        .filter(Column::Token.eq(token));
    if !actor.role.is_admin() {
        update = update.filter(Column::CreatedBy.eq(&actor.sub));
    }
    let result = update.exec(&*DB).await?;
    if result.rows_affected == 0 {
        anyhow::bail!("invitation not found or not yours");
    }
    Ok(())
}

/// Public view of an invitation for an invitee opening the link.
pub async fn prompt(token: &str) -> anyhow::Result<InviteePrompt> {
    let now = now();
    let Some(m) = Invitations::find_by_id(token.to_owned()).one(&*DB).await? else {
        return Ok(InviteePrompt::Unavailable(Unavailable::NotFound));
    };
    let max_uses = read_max_uses(m.max_uses);
    let used = read_count(m.accounts_created);
    Ok(match status(now, m.expires_at, m.revoked, max_uses, used) {
        InviteStatus::Active => InviteePrompt::Open,
        InviteStatus::Expired => InviteePrompt::Unavailable(Unavailable::Expired),
        InviteStatus::Revoked => InviteePrompt::Unavailable(Unavailable::Revoked),
        InviteStatus::Exhausted => InviteePrompt::Unavailable(Unavailable::Exhausted),
    })
}

/// Why a redeem attempt failed, in terms the signup endpoint can turn into
/// invitee-facing feedback. `UsernameTaken` is split out because it is the
/// common, user-correctable case.
pub enum RedeemError {
    /// Submitted form data was rejected (bad username, missing field).
    Invalid(String),
    /// The invitation is no longer usable (expired, revoked, exhausted, gone).
    Unavailable,
    /// The chosen username already exists in kanidm.
    UsernameTaken,
    /// An unexpected server/database/kanidm error.
    Internal(anyhow::Error),
}

/// Redeem an invitation: reserve a use, provision the kanidm person, and mint
/// the reset token — all inside one transaction so the max-uses cap is exact.
/// Returns the kanidm reset URL to redirect the invitee to.
pub async fn redeem(token: &str, form: SignupForm) -> Result<String, RedeemError> {
    // Parse external input into strong types before touching the database.
    let username = ValidUsername::parse(&form.username).map_err(RedeemError::Invalid)?;
    let displayname = form.displayname.trim();
    if displayname.is_empty() {
        return Err(RedeemError::Invalid("display name is required".into()));
    }
    let email = form.email.trim();
    if email.is_empty() {
        return Err(RedeemError::Invalid("email is required".into()));
    }

    let txn = DB.begin().await.map_err(|e| RedeemError::Internal(e.into()))?;
    let result = redeem_in_txn(&txn, token, &username, displayname, email).await;
    match result {
        Ok(reset_url) => {
            txn.commit().await.map_err(|e| RedeemError::Internal(e.into()))?;
            Ok(reset_url)
        }
        Err(e) => {
            let _ = txn.rollback().await;
            Err(e)
        }
    }
}

/// Reserve a use as a single conditional UPDATE: it increments the counter only
/// while the invitation is still valid. Being one atomic write, it acquires
/// SQLite's write lock and re-reads the count fresh, so the cap is exact even
/// under concurrent redeems and SQLite's deferred BEGIN. Returns whether a use
/// was reserved (false = the invitation is not currently valid).
async fn reserve_use<C: ConnectionTrait>(conn: &C, token: &str, now: i64) -> Result<bool, DbErr> {
    let reserved = Invitations::update_many()
        .col_expr(
            Column::AccountsCreated,
            Expr::col(Column::AccountsCreated).add(1),
        )
        .filter(Column::Token.eq(token))
        .filter(Column::Revoked.eq(false))
        .filter(
            Condition::any()
                .add(Column::ExpiresAt.is_null())
                .add(Column::ExpiresAt.gt(now)),
        )
        .filter(
            Condition::any()
                .add(Column::MaxUses.is_null())
                .add(Expr::col(Column::AccountsCreated).lt(Expr::col(Column::MaxUses))),
        )
        .exec(conn)
        .await?;
    Ok(reserved.rows_affected > 0)
}

async fn redeem_in_txn(
    txn: &DatabaseTransaction,
    token: &str,
    username: &ValidUsername,
    displayname: &str,
    email: &str,
) -> Result<String, RedeemError> {
    let reserved = reserve_use(txn, token, now())
        .await
        .map_err(|e| RedeemError::Internal(e.into()))?;
    if !reserved {
        return Err(RedeemError::Unavailable);
    }

    // Provision the account and mint the reset token while the reservation is
    // still uncommitted. kanidm owns uniqueness, so a duplicate username
    // surfaces here as UsernameTaken; any error rolls the transaction back,
    // releasing the use.
    let kanidm = Kanidm::new().map_err(RedeemError::Internal)?;
    kanidm
        .create_person(username.as_str(), displayname, email)
        .await
        .map_err(|e| match e {
            CreatePersonError::UsernameTaken => RedeemError::UsernameTaken,
            CreatePersonError::Other(err) => RedeemError::Internal(err),
        })?;
    let intent = kanidm
        .credential_update_intent(username.as_str())
        .await
        .map_err(RedeemError::Internal)?;

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
        assert_eq!(status(100, Some(200), false, nz(5), 2), InviteStatus::Active);
    }

    #[test]
    fn no_cap_is_never_exhausted() {
        assert_eq!(status(100, Some(200), false, None, 9_999), InviteStatus::Active);
    }

    #[test]
    fn no_expiry_is_never_expired() {
        assert_eq!(status(i64::MAX, None, false, None, 0), InviteStatus::Active);
    }

    #[test]
    fn exhausted_at_or_above_cap() {
        assert_eq!(status(100, Some(200), false, nz(3), 3), InviteStatus::Exhausted);
        assert_eq!(status(100, Some(200), false, nz(3), 4), InviteStatus::Exhausted);
    }

    #[test]
    fn expired_when_now_reaches_expiry() {
        assert_eq!(status(200, Some(200), false, None, 0), InviteStatus::Expired);
        assert_eq!(status(201, Some(200), false, None, 0), InviteStatus::Expired);
    }

    #[test]
    fn revoked_takes_precedence_over_expired_and_exhausted() {
        assert_eq!(status(300, Some(200), true, nz(1), 5), InviteStatus::Revoked);
    }

    #[test]
    fn expired_takes_precedence_over_exhausted() {
        assert_eq!(status(300, Some(200), false, nz(1), 5), InviteStatus::Expired);
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

    /// A fresh in-memory database with migrations applied.
    async fn mem_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        db
    }

    fn invitation(token: &str, max_uses: Option<i64>, expires_at: Option<i64>, revoked: bool) -> ActiveModel {
        ActiveModel {
            token: Set(token.to_owned()),
            label: Set("label".to_owned()),
            created_by: Set("owner".to_owned()),
            created_at: Set(0),
            expires_at: Set(expires_at),
            max_uses: Set(max_uses),
            accounts_created: Set(0),
            revoked: Set(revoked),
        }
    }

    async fn count(db: &DatabaseConnection, token: &str) -> i64 {
        Invitations::find_by_id(token.to_owned())
            .one(db)
            .await
            .unwrap()
            .unwrap()
            .accounts_created
    }

    #[tokio::test]
    async fn migration_and_entity_round_trip() {
        let db = mem_db().await;
        invitation("t", Some(2), Some(9_999_999_999), false).insert(&db).await.unwrap();
        let got = Invitations::find_by_id("t".to_owned()).one(&db).await.unwrap().unwrap();
        assert_eq!(got.max_uses, Some(2));
        assert_eq!(got.accounts_created, 0);
        assert!(!got.revoked);
    }

    #[tokio::test]
    async fn reserve_use_stops_at_the_cap() {
        let db = mem_db().await;
        invitation("t", Some(2), Some(9_999_999_999), false).insert(&db).await.unwrap();
        assert!(reserve_use(&db, "t", 100).await.unwrap());
        assert!(reserve_use(&db, "t", 100).await.unwrap());
        // Third attempt is over the cap: no reservation, counter unchanged.
        assert!(!reserve_use(&db, "t", 100).await.unwrap());
        assert_eq!(count(&db, "t").await, 2);
    }

    #[tokio::test]
    async fn reserve_use_allows_unlimited() {
        let db = mem_db().await;
        invitation("t", None, Some(9_999_999_999), false).insert(&db).await.unwrap();
        for _ in 0..5 {
            assert!(reserve_use(&db, "t", 100).await.unwrap());
        }
        assert_eq!(count(&db, "t").await, 5);
    }

    #[tokio::test]
    async fn reserve_use_allows_non_expiring() {
        let db = mem_db().await;
        invitation("t", None, None, false).insert(&db).await.unwrap();
        assert!(reserve_use(&db, "t", i64::MAX).await.unwrap());
        assert_eq!(count(&db, "t").await, 1);
    }

    #[tokio::test]
    async fn reserve_use_rejects_revoked_and_expired() {
        let db = mem_db().await;
        invitation("revoked", None, Some(9_999_999_999), true).insert(&db).await.unwrap();
        invitation("expired", None, Some(50), false).insert(&db).await.unwrap();
        assert!(!reserve_use(&db, "revoked", 100).await.unwrap());
        assert!(!reserve_use(&db, "expired", 100).await.unwrap());
        assert!(!reserve_use(&db, "missing", 100).await.unwrap());
    }
}
