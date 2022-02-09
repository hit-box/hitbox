//! Structures and traits for custom backend development and testing process.
mod mock_adapter;
pub mod mock_backend;

pub use hitbox_backend::{
    Backend, BackendError, CacheBackend, Delete, DeleteStatus, Get, Lock, LockStatus, Set,
};
// pub use mock_adapter::MockAdapter;
