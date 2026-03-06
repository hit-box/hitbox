#![doc = include_str!("../README.md")]

pub(crate) mod decode;
mod error;
mod extractor;
mod fields;
mod frame;
mod operation;
mod pool;
mod predicate;
mod value;

pub use error::ProtoError;
pub use extractor::{ProtoFieldsExtract, ProtoFieldsExtractor};
pub use fields::{FieldSpec, FieldsBuilder};
pub use frame::{FrameDecoder, NoFraming};
pub use operation::{ElementMatcher, Operation, ValueMatcher};
pub use pool::DescriptorPool;
pub use predicate::{ProtoFields, ProtoFieldsPredicate};
pub use value::ProtoValue;
