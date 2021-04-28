mod backend;
mod adapter;

pub use adapter::MockAdapter;
pub use backend::{Backend, BackendError, Get, Set, Lock, Delete, LockStatus};
