use hitbox_http::CacheableHttpRequest;
use hitbox::predicates::Predicate;
use hitbox_http::predicates::{NeutralPredicate, query::QueryPredicate};

pub enum RequestPredicate {
    Query { key: String, value: String },
}

pub struct EndpointConfig {
    pub request_predicates: Vec<RequestPredicate>,
}

impl EndpointConfig {
    pub fn create<ReqBody>(&self) -> Box<dyn Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync> 
        where
            ReqBody: Send + 'static,
    {
        let acc_predicate = Box::new(NeutralPredicate::new());
       self 
            .request_predicates
            .iter()
            .rfold(acc_predicate, |inner, predicate| match predicate {
                RequestPredicate::Query { key, value } => Box::new(inner.query(key.clone(), value.clone()))
            })
    }
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            request_predicates: vec![ RequestPredicate::Query { key: String::from("balala"), value: String::from("true") }],
        }
    }
}
