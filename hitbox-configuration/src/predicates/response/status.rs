use std::num::NonZeroU16;

use hitbox_core::Predicate;
use hitbox_http::predicates::response::{StatusClass, StatusCode};
use hitbox_http::{CacheableHttpResponse, FromBytes};
use http::StatusCode as HttpStatusCode;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

type CorePredicate<ReqBody> =
    Box<dyn Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Class {
    Explicit { class: StatusClass },
    Implicit(StatusClass),
}

impl Class {
    fn class(&self) -> StatusClass {
        match self {
            Class::Explicit { class } => *class,
            Class::Implicit(cls) => *cls,
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct Range {
    range: [NonZeroU16; 2],
}

impl Range {
    fn start(&self) -> NonZeroU16 {
        self.range[0]
    }

    fn end(&self) -> NonZeroU16 {
        self.range[1]
    }
}

impl<'de> Deserialize<'de> for Range {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let range: [NonZeroU16; 2] = Deserialize::deserialize(deserializer)?;
        let [start, end] = range;

        if start.get() > end.get() {
            return Err(D::Error::custom(format!(
                "Invalid status code range: start ({}) must be less than or equal to end ({})",
                start, end
            )));
        }

        Ok(Range { range })
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Operation {
    // Explicit-only forms (must be objects with named fields)
    Range { range: Range },

    // Forms with both explicit and implicit syntax
    Class(Class),
    Eq(Eq),
    In(In),  // Must be last: implicit form matches any array
}

impl Operation {
    pub fn into_predicates<ReqBody>(
        &self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Operation::Eq(eq) => {
                Box::new(StatusCode::new(inner, eq.status().get().try_into().unwrap()))
            }
            Operation::In(r#in) => {
                let status_codes: Vec<HttpStatusCode> = r#in.statuses()
                    .iter()
                    .map(|c| c.get().try_into().unwrap())
                    .collect();
                Box::new(StatusCode::new_in(inner, status_codes))
            }
            Operation::Range { range } => {
                Box::new(StatusCode::new_range(
                    inner,
                    range.start().get().try_into().unwrap(),
                    range.end().get().try_into().unwrap(),
                ))
            }
            Operation::Class(class) => Box::new(StatusCode::new_class(inner, class.class())),
        }
    }
}
