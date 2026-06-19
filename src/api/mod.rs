//! Server functions: the typed RPC surface the client calls. Bodies run on the
//! server; the client gets generated stubs. Grouped by domain.

mod dashboard;
mod invites;
mod signup;

pub use dashboard::get_dashboard;
pub use invites::{create_invite, revoke_invite};
pub use signup::{get_invite, signup};
