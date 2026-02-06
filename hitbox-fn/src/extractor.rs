//! Cache key extraction for function arguments.

use std::marker::PhantomData;

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use crate::Args;

/// Trait for types that can contribute to a cache key.
///
/// Implement this trait for your custom types to control how they appear in cache keys.
/// Blanket implementations are provided for common primitive types.
///
/// # Example
///
/// ```
/// use hitbox_fn::KeyExtract;
/// use hitbox_core::KeyPart;
///
/// struct UserId(u64);
///
/// impl KeyExtract for UserId {
///     fn extract(&self) -> Vec<KeyPart> {
///         vec![KeyPart::new("user_id", Some(self.0.to_string()))]
///     }
/// }
/// ```
///
/// # Automatic composition
///
/// When all elements of a tuple implement `KeyExtract`, the tuple automatically
/// implements it too by concatenating all key parts:
///
/// ```
/// use hitbox_fn::{Args, KeyExtract};
/// use hitbox_core::KeyPart;
///
/// struct UserId(u64);
/// impl KeyExtract for UserId {
///     fn extract(&self) -> Vec<KeyPart> {
///         vec![KeyPart::new("user_id", Some(self.0.to_string()))]
///     }
/// }
///
/// struct TenantId(String);
/// impl KeyExtract for TenantId {
///     fn extract(&self) -> Vec<KeyPart> {
///         vec![KeyPart::new("tenant", Some(self.0.clone()))]
///     }
/// }
///
/// let args = Args((UserId(42), TenantId("acme".into())));
/// let parts = args.extract();
/// assert_eq!(parts.len(), 2);
/// ```
pub trait KeyExtract {
    /// Extract key parts from this value.
    fn extract(&self) -> Vec<KeyPart>;
}

// KeyExtract implementations for primitive types

impl KeyExtract for u8 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("u8", Some(self.to_string()))]
    }
}

impl KeyExtract for u16 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("u16", Some(self.to_string()))]
    }
}

impl KeyExtract for u32 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("u32", Some(self.to_string()))]
    }
}

impl KeyExtract for u64 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("u64", Some(self.to_string()))]
    }
}

impl KeyExtract for u128 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("u128", Some(self.to_string()))]
    }
}

impl KeyExtract for usize {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("usize", Some(self.to_string()))]
    }
}

impl KeyExtract for i8 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("i8", Some(self.to_string()))]
    }
}

impl KeyExtract for i16 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("i16", Some(self.to_string()))]
    }
}

impl KeyExtract for i32 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("i32", Some(self.to_string()))]
    }
}

impl KeyExtract for i64 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("i64", Some(self.to_string()))]
    }
}

impl KeyExtract for i128 {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("i128", Some(self.to_string()))]
    }
}

impl KeyExtract for isize {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("isize", Some(self.to_string()))]
    }
}

impl KeyExtract for bool {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("bool", Some(self.to_string()))]
    }
}

impl KeyExtract for String {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("str", Some(self.clone()))]
    }
}

impl KeyExtract for &str {
    fn extract(&self) -> Vec<KeyPart> {
        vec![KeyPart::new("str", Some(self.to_string()))]
    }
}

impl<T: KeyExtract> KeyExtract for Option<T> {
    fn extract(&self) -> Vec<KeyPart> {
        match self {
            Some(v) => v.extract(),
            None => vec![KeyPart::new("none", None::<&str>)],
        }
    }
}

impl<T: KeyExtract + ?Sized> KeyExtract for &T {
    fn extract(&self) -> Vec<KeyPart> {
        (*self).extract()
    }
}

// KeyExtract implementations for Args<tuples>

impl KeyExtract for Args<()> {
    fn extract(&self) -> Vec<KeyPart> {
        vec![]
    }
}

impl<T0: KeyExtract> KeyExtract for Args<(T0,)> {
    fn extract(&self) -> Vec<KeyPart> {
        self.0.0.extract()
    }
}

impl<T0: KeyExtract, T1: KeyExtract> KeyExtract for Args<(T0, T1)> {
    fn extract(&self) -> Vec<KeyPart> {
        let mut parts = self.0.0.extract();
        parts.extend(self.0.1.extract());
        parts
    }
}

impl<T0: KeyExtract, T1: KeyExtract, T2: KeyExtract> KeyExtract for Args<(T0, T1, T2)> {
    fn extract(&self) -> Vec<KeyPart> {
        let mut parts = self.0.0.extract();
        parts.extend(self.0.1.extract());
        parts.extend(self.0.2.extract());
        parts
    }
}

impl<T0: KeyExtract, T1: KeyExtract, T2: KeyExtract, T3: KeyExtract> KeyExtract
    for Args<(T0, T1, T2, T3)>
{
    fn extract(&self) -> Vec<KeyPart> {
        let mut parts = self.0.0.extract();
        parts.extend(self.0.1.extract());
        parts.extend(self.0.2.extract());
        parts.extend(self.0.3.extract());
        parts
    }
}

impl<T0: KeyExtract, T1: KeyExtract, T2: KeyExtract, T3: KeyExtract, T4: KeyExtract> KeyExtract
    for Args<(T0, T1, T2, T3, T4)>
{
    fn extract(&self) -> Vec<KeyPart> {
        let mut parts = self.0.0.extract();
        parts.extend(self.0.1.extract());
        parts.extend(self.0.2.extract());
        parts.extend(self.0.3.extract());
        parts.extend(self.0.4.extract());
        parts
    }
}

impl<T0: KeyExtract, T1: KeyExtract, T2: KeyExtract, T3: KeyExtract, T4: KeyExtract, T5: KeyExtract>
    KeyExtract for Args<(T0, T1, T2, T3, T4, T5)>
{
    fn extract(&self) -> Vec<KeyPart> {
        let mut parts = self.0.0.extract();
        parts.extend(self.0.1.extract());
        parts.extend(self.0.2.extract());
        parts.extend(self.0.3.extract());
        parts.extend(self.0.4.extract());
        parts.extend(self.0.5.extract());
        parts
    }
}

// FnExtractor - bridges KeyExtract to Extractor

/// Generic extractor that uses [`KeyExtract`] trait to generate cache keys.
///
/// This extractor bridges the simple `KeyExtract` trait to hitbox's `Extractor` trait,
/// adding the function path as a prefix to ensure different functions have different cache keys.
///
/// # Example
///
/// ```
/// use hitbox_fn::{Args, FnExtractor};
///
/// let extractor = FnExtractor::<Args<(u64, String)>>::new("my_module::fetch_user");
/// ```
pub struct FnExtractor<T> {
    fn_path: &'static str,
    // Use fn() -> T instead of T to avoid requiring T: 'static for FnExtractor<T>: 'static.
    // This allows FnExtractor to work with types containing non-'static lifetimes.
    _marker: PhantomData<fn() -> T>,
}

impl<T> FnExtractor<T> {
    /// Create a new function extractor with the given fully qualified function path.
    ///
    /// The function path is used as a prefix in the cache key to ensure different
    /// functions produce different keys even with the same arguments.
    pub fn new(fn_path: &'static str) -> Self {
        Self {
            fn_path,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<T> Extractor for FnExtractor<T>
where
    T: KeyExtract + Send + Sync,
{
    type Subject = T;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let extracted = subject.extract();
        let mut parts = KeyParts::new(subject);
        parts.push(KeyPart::new("fn", Some(self.fn_path)));
        for part in extracted {
            parts.push(part);
        }
        parts
    }
}
