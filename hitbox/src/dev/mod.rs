mod adapter;
mod backend;

// pub use adapter::MockAdapter;
pub use backend::{Backend, BackendError, Delete, Get, Lock, LockStatus, Set};
