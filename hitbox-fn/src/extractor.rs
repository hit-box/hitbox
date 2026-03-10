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
/// use hitbox::KeyPart;
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
/// use hitbox::KeyPart;
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

macro_rules! impl_key_extract_for_scalar {
    ($($ty:ty => $name:expr),* $(,)?) => {
        $(
            impl KeyExtract for $ty {
                fn extract(&self) -> Vec<KeyPart> {
                    vec![KeyPart::new($name, Some(self.to_string()))]
                }
            }
        )*
    };
}

impl_key_extract_for_scalar! {
    u8 => "u8", u16 => "u16", u32 => "u32", u64 => "u64", u128 => "u128", usize => "usize",
    i8 => "i8", i16 => "i16", i32 => "i32", i64 => "i64", i128 => "i128", isize => "isize",
    bool => "bool",
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

macro_rules! impl_key_extract_for_args {
    () => {
        impl KeyExtract for Args<()> {
            fn extract(&self) -> Vec<KeyPart> {
                vec![]
            }
        }
    };
    ($first:tt : $T0:ident $(, $idx:tt : $T:ident)*) => {
        impl<$T0: KeyExtract $(, $T: KeyExtract)*> KeyExtract for Args<($T0, $($T,)*)> {
            fn extract(&self) -> Vec<KeyPart> {
                [self.0.$first.extract() $(, self.0.$idx.extract())*]
                    .into_iter()
                    .flatten()
                    .collect()
            }
        }
    };
}

impl_key_extract_for_args!();
impl_key_extract_for_args!(0: T0);
impl_key_extract_for_args!(0: T0, 1: T1);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10);
impl_key_extract_for_args!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5, 6: T6, 7: T7, 8: T8, 9: T9, 10: T10, 11: T11);

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
