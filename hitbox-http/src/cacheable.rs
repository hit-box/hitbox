use hyper::body::Body as HttpBody;

use crate::BufferedBody;

/// Enables predicates and extractors to work uniformly with requests and responses.
///
/// This trait abstracts over the common pattern of decomposing an HTTP message into
/// its metadata (headers, status, method, etc.) and body, then reconstructing it.
/// Predicates that inspect the body use this to temporarily take ownership of the
/// body, examine it, and return a potentially modified subject.
///
/// # For Implementors
///
/// Implementations must ensure round-trip consistency: calling `from_parts` with
/// the result of `into_parts` must produce an equivalent subject.
///
/// ```
/// use hitbox_http::CacheableSubject;
///
/// fn round_trip<S: CacheableSubject>(subject: S) -> S {
///     let (parts, body) = subject.into_parts();
///     S::from_parts(parts, body)
///     // reconstructed should be equivalent to subject
/// }
/// ```
///
/// # For Callers
///
/// Use this trait when writing predicates or extractors that need to:
/// - Inspect the body without fully consuming it
/// - Pass the subject through a chain of operations
/// - Work generically with both requests and responses
///
/// # Caveats
///
/// After `into_parts`, the body may be in a different state than before. If a
/// predicate consumed bytes, the body transitions from `Passthrough` to `Partial`
/// or `Complete`. Callers must handle all [`BufferedBody`] states.
pub trait CacheableSubject {
    /// The HTTP body type.
    type Body: HttpBody;

    /// The metadata type (e.g., [`http::request::Parts`] or [`http::response::Parts`]).
    type Parts;

    /// Decomposes this subject into metadata and body.
    ///
    /// After this call, the body may be in any [`BufferedBody`] state depending
    /// on prior operations.
    fn into_parts(self) -> (Self::Parts, BufferedBody<Self::Body>);

    /// Reconstructs a subject from metadata and body.
    ///
    /// This is the inverse of [`into_parts`](Self::into_parts).
    fn from_parts(parts: Self::Parts, body: BufferedBody<Self::Body>) -> Self;
}
