mod base;
mod successful;
mod error;
mod error_with_stale;

pub use base::{UpstreamPolled, UpstreamPolledStaleRetrieved};
pub use successful::UpstreamPolledSuccessful;
pub use error::UpstreamPolledError;
pub use error_with_stale::UpstreamPolledErrorStaleRetrieved;
