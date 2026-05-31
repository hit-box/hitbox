#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod backend;
pub mod builder;
pub mod error;

pub use backend::S3Backend;
pub use builder::S3BackendBuilder;
pub use error::S3Error;
