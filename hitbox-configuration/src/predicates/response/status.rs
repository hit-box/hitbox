use std::num::NonZeroU16;

use hitbox_core::Predicate;
use hitbox_http::predicates::response::{StatusClass as HttpStatusClass, StatusCode};
use hitbox_http::{CacheableHttpResponse, FromBytes};
use http::StatusCode as HttpStatusCode;
use hyper::body::Body as HttpBody;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// Configuration version of StatusClass with JsonSchema support
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, JsonSchema)]
pub enum StatusClass {
    Informational,
    Success,
    Redirect,
    ClientError,
    ServerError,
}

impl From<StatusClass> for HttpStatusClass {
    fn from(class: StatusClass) -> Self {
        match class {
            StatusClass::Informational => HttpStatusClass::Informational,
            StatusClass::Success => HttpStatusClass::Success,
            StatusClass::Redirect => HttpStatusClass::Redirect,
            StatusClass::ClientError => HttpStatusClass::ClientError,
            StatusClass::ServerError => HttpStatusClass::ServerError,
        }
    }
}

type CorePredicate<ReqBody> =
    Box<dyn Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum Eq {
    Explicit { eq: NonZeroU16 },
    Implicit(NonZeroU16),
}

impl Eq {
    fn status(&self) -> NonZeroU16 {
        match self {
            Eq::Explicit { eq } => *eq,
            Eq::Implicit(val) => *val,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum In {
    Explicit { r#in: Vec<NonZeroU16> },
    Implicit(Vec<NonZeroU16>),
}

impl In {
    fn statuses(&self) -> &[NonZeroU16] {
        match self {
            In::Explicit { r#in } => r#in,
            In::Implicit(vals) => vals,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum Class {
    Explicit { class: StatusClass },
    Implicit(StatusClass),
}

impl Class {
    fn class(&self) -> HttpStatusClass {
        match self {
            Class::Explicit { class } => (*class).into(),
            Class::Implicit(cls) => (*cls).into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, JsonSchema)]
#[serde(transparent)]
pub struct Range {
    pub range: [NonZeroU16; 2],
}

impl Range {
    fn start(&self) -> NonZeroU16 {
        self.range[0]
    }

    fn end(&self) -> NonZeroU16 {
        self.range[1]
    }

    fn validate(&self) -> Result<(), String> {
        let [start, end] = self.range;
        if start.get() > end.get() {
            return Err(format!(
                "Invalid status code range: start ({}) must be less than or equal to end ({})",
                start, end
            ));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum Operation {
    // Explicit-only forms (must be objects with named fields)
    Range { range: Range },

    // Forms with both explicit and implicit syntax
    Class(Class),
    Eq(Eq),
    In(In), // Must be last: implicit form matches any array
}

impl Operation {
    pub fn into_predicates<ReqBody>(
        &self,
        inner: CorePredicate<ReqBody>,
    ) -> Result<CorePredicate<ReqBody>, ConfigError>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Operation::Eq(eq) => {
                let status = parse_status_code(eq.status().get())?;
                Ok(Box::new(StatusCode::new(inner, status)))
            }
            Operation::In(r#in) => {
                let status_codes = parse_status_codes(r#in.statuses())?;
                Ok(Box::new(StatusCode::new_in(inner, status_codes)))
            }
            Operation::Range { range } => {
                range.validate().map_err(|e| ConfigError::InvalidConfiguration(e))?;
                let start = parse_status_code(range.start().get())?;
                let end = parse_status_code(range.end().get())?;
                Ok(Box::new(StatusCode::new_range(inner, start, end)))
            }
            Operation::Class(class) => Ok(Box::new(StatusCode::new_class(inner, class.class()))),
        }
    }
}

fn parse_status_code(code: u16) -> Result<HttpStatusCode, ConfigError> {
    HttpStatusCode::from_u16(code).map_err(|_| ConfigError::InvalidStatusCode(code))
}

fn parse_status_codes(codes: &[NonZeroU16]) -> Result<Vec<HttpStatusCode>, ConfigError> {
    codes.iter().map(|c| parse_status_code(c.get())).collect()
}
