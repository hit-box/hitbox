//! hitbox [Backend] implementation for Tarantool.
//! [Backend]: hitbox_backend::Backend
pub mod backend;

#[doc(inline)]
pub use crate::backend::{TarantoolBackend, TarantoolBackendBuilder};
