# Context: Invites

An app that lets existing users issue invitation links so new people can
self-provision a kanidm account.

## Ubiquitous Language

### Inviter
An existing kanidm user, authenticated to this app via OAuth/OIDC against
kanidm. The only actor who can create [Invitations](#invitation). Not a
distinct kanidm entity — just a person account that has logged in here.

### Invitation
A shareable link that authorizes account self-provisioning. Created by and
**owned by** an [Inviter](#inviter), who chooses its expiration at creation.
An Inviter sees and revokes only their own Invitations; an
[Administrator](#administrator) sees and revokes all. Carries:
- an **expiration** — an absolute time after which it is no longer
  [valid](#valid-invitation);
- an optional **max-uses cap** — a limit on how many accounts it may
  provision (e.g. single-use); unlimited if unset;
- an **accounts-created counter** (see below);
- a **revoked** state — an [Inviter](#inviter) may revoke it early.

Multi-use by default: a single Invitation may provision many accounts while it
remains [valid](#valid-invitation).

### Valid Invitation
An [Invitation](#invitation) that may currently provision an account. Valid
iff ALL hold: not past its expiration, not revoked, and (no max-uses cap OR
accounts-created counter is below the cap). Validity is checked both when the
[Invitee](#invitee) loads the form and again on submission.

### Administrator
An [Inviter](#inviter) who is a member of kanidm's admin group (`idm_admins`).
Distinguished only by that group membership, surfaced to this app as a group
claim in the OIDC token (requires a kanidm scope map). An Administrator may
view and revoke any [Invitation](#invitation), not just their own. Invite
*creation* is open to all authenticated users and is NOT gated on this role.

### Invitee
A person who follows a [valid Invitation](#valid-invitation) and provisions a
kanidm account through it. Not authenticated to this app — known only by the
account details they submit: a username (kanidm `name`), a display name
(kanidm `displayname`), and a required email (kanidm `mail`, unverified).
Becomes a kanidm person on submission, then is handed off (see
[Credential handoff](#credential-handoff)).

### Accounts-created counter
The count an [Invitation](#invitation) keeps of kanidm persons provisioned
through it. Counts **persons created** (incremented when the kanidm person is
created), NOT completed signups. A person who is created but never sets a
credential ("ghost") is still counted. See ADR on credential handoff.

### Credential handoff
After this app creates the kanidm person, it does NOT collect or set the
password. It mints a kanidm credential-update intent token and redirects the
[Invitee](#invitee) to kanidm's own credential reset page, where they set
their password (and any passkey/MFA). This app never custodies a plaintext
password.
