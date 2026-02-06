//! Upstream adapter for async functions.

use std::future::Future;

use hitbox_core::Upstream;

/// Wrapper that adapts an async function to the [`Upstream`] trait.
///
/// This allows any async function or closure to be used as the upstream
/// data source in the hitbox caching FSM.
///
/// # Example
///
/// ```
/// use hitbox_fn::FnUpstream;
///
/// async fn fetch_data(id: u64) -> String {
///     format!("data_{id}")
/// }
///
/// let upstream = FnUpstream::new(fetch_data);
/// ```
pub struct FnUpstream<F> {
    func: F,
}

impl<F> FnUpstream<F> {
    /// Create a new function upstream wrapper.
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F, Args, Fut, Res> Upstream<Args> for FnUpstream<F>
where
    F: FnMut(Args) -> Fut,
    Fut: Future<Output = Res> + Send,
{
    type Response = Res;
    type Future = Fut;

    fn call(mut self, req: Args) -> Self::Future {
        (self.func)(req)
    }
}
