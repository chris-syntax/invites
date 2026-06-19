# ADR 0001 — Data layer: SeaORM with migrations

- Status: Accepted
- Date: 2026-06-18

## Context

The app persists invitations in SQLite. The first cut used `sqlx` directly with
runtime queries and hand-written `row.get("col")` extraction, and created the
schema with an inline `CREATE TABLE IF NOT EXISTS` on first pool access. Two
problems:

1. No typed models — every read re-derived columns by string key.
2. No migration story — `IF NOT EXISTS` cannot evolve an existing schema.

The server is async end-to-end (tokio, axum, reqwest). Critically, `redeem`
holds a database transaction open across an async call to kanidm so the
max-uses cap stays exact.

## Decision

Use **SeaORM** with **sea-orm-migration**.

- SeaORM is an async-native ORM built on `sqlx`, with SQLite support — it fits
  the async stack and keeps the transaction-across-await pattern in `redeem`.
- Typed entity models (`server::entity::invitations::Model` / `ActiveModel`)
  replace `row.get` plumbing.
- Schema lives in versioned migrations under `server::migration`, run on
  startup via the `Migrator`.

## Alternatives considered

- **Diesel** — rejected. Synchronous; `diesel-async` does not support SQLite, so
  it would force `spawn_blocking` and break the `redeem` flow (can't `.await` the
  async kanidm client while holding a Diesel transaction).
- **sqlx `FromRow` + `sqlx::migrate!`** — lighter (typed structs, real
  migrations, no new dependency) but not an ORM. Rejected because proper ORM
  models were a goal.

## Consequences

- The ORM models *storage*. The cross-stack wire types in `shared.rs`
  (`InvitationView`, `InviteePrompt`) remain the API contract; `db.rs` maps
  entity → view, where `status()` lives.
- `redeem` reserves a use with a single conditional `UPDATE` (atomic, so the cap
  is exact even under SQLite's deferred `BEGIN`); a kanidm failure rolls the
  transaction back, consuming no use.
- A full ORM is heavy for one table today, accepted as the schema will grow.
