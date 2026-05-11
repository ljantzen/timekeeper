/// Well-known UUID for the default single-user mode.
/// Stable across machines; future multi-user sets a real user_id via config.
pub const LOCAL_USER_ID: &str = "00000000-0000-0000-0000-000000000001";

pub mod entry;
pub mod project;
pub mod task;
pub mod user;
