//! Structures and traits for custom backend development and testing process.
mod mock_adapter;
pub mod mock_backend;

pub use mock_adapter::MockAdapter;
pub use hitbox_backend::{Backend, BackendError, Delete, DeleteStatus, Get, Lock, LockStatus, Set};
