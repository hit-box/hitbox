mod mock_adapter;
mod mock_backend;

// pub use adapter::MockAdapter;
pub use mock_backend::{Backend, BackendError, Delete, Get, Lock, LockStatus, Set};
