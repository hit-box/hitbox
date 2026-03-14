#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub mod cacheable;
pub mod config;
pub mod context;
pub mod extractor;
pub mod key;
pub mod label;
pub mod offload;
pub mod policy;
pub mod predicate;
pub mod request;
pub mod response;
pub mod upstream;
pub mod value;

pub use cacheable::Cacheable;
pub use config::{CacheConfig, CacheConfigs};
pub use context::{
    BoxContext, CacheContext, CacheStatus, CacheStatusExt, CacheTiming, Context, ForwardReason,
    ReadMode, ResponseSource, finalize_context,
};
pub use extractor::Extractor;
pub use key::{CacheKey, KeyPart, KeyParts};
pub use label::BackendLabel;
pub use offload::{DisabledOffload, Offload, OffloadKey};
pub use policy::{
    CacheBehaviorPolicy, CachePolicy, ConcurrencyLimit, EnabledCacheConfig, EntityPolicyConfig,
    PolicyConfig, PolicyConfigBuilder, StalePolicy,
};
pub use predicate::{And, Neutral, Not, Or, Predicate, PredicateExt, PredicateResult};
pub use request::{CacheablePolicyData, CacheableRequest, RequestCachePolicy};
pub use response::{CacheState, CacheableResponse, ResponseCachePolicy};
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
pub use smallbox::space::S4;
#[doc(hidden)]
pub use smol_str::SmolStr;
pub use upstream::Upstream;
pub use value::CacheValue;

/// Raw byte data type used for serialized cache values.
/// Using `Bytes` provides efficient zero-copy cloning via reference counting.
pub type Raw = bytes::Bytes;
