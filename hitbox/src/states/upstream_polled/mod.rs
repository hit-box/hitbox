mod base;
mod error;
mod error_with_stale;
mod successful;

pub use base::{UpstreamPolled, UpstreamPolledStaleRetrieved};
pub use error::UpstreamPolledError;
pub use error_with_stale::UpstreamPolledErrorStaleRetrieved;
pub use successful::UpstreamPolledSuccessful;
