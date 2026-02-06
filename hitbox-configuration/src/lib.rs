#![doc = include_str!("../README.md")]

pub use predicates::{request::Request, response::Response};

pub mod backend;
pub mod config;
pub mod endpoint;
pub mod error;
pub mod extractors;
pub mod predicates;
pub mod types;

pub use backend::Backend;
pub use config::{ConfigEndpoint, ConfigRoute};
pub use endpoint::{
    Endpoint, EndpointBuilder, RequestExtractor, RequestPredicate, ResponsePredicate,
    RoutedEndpoint,
};
pub use error::{ConfigError, parse_config};
