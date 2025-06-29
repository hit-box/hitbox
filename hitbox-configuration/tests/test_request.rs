use bytes::Bytes;
use hitbox::predicate::PredicateResult;
use hitbox_configuration::{
    Endpoint,
    predicates::request::{Expression, Operation, Predicate, QueryOperation, Request},
};
use hitbox_http::CacheableHttpRequest;
use hitbox_http::predicates::NeutralRequestPredicate;
use http::Request as HttpRequest;
use http_body_util::Empty;
use pretty_assertions::assert_eq;

#[test]
fn test_expression_tree_serialize() {
    let query_params = vec![("cache".to_owned(), "true".to_owned())]
        .into_iter()
        .collect();
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let method_post = Expression::Predicate(Predicate::Method("POST".to_owned()));
    let path = Expression::Predicate(Predicate::Path("/books".to_owned()));
    let query = Expression::Predicate(Predicate::Query(QueryOperation::Eq(query_params)));
    let and_ = Expression::Operation(Operation::And(vec![method_get.into(), path.into()]));
    let or_ = Expression::Operation(Operation::Or(vec![
        query.into(),
        method_post.into(),
        and_.into(),
    ]));
    let request = Request::Tree(or_);
    let endpoint = Endpoint { request };
    let yaml_str = serde_yaml::to_string(&endpoint).unwrap();
    let expected = r"request:
  Or:
  - Query:
      operation: Eq
      cache: 'true'
  - Method: POST
  - And:
    - Method: GET
    - Path: /books
";
    assert_eq!(yaml_str, expected);
}

#[tokio::test]
async fn test_expression_into_predicates() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let method_post = Expression::Predicate(Predicate::Method("POST".to_owned()));
    let method_head = Expression::Predicate(Predicate::Method("HEAD".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![
        method_get.into(),
        method_post.into(),
        method_head.into(),
    ]));
    let predicate_or = or_.into_predicates(inner);
    dbg!(&predicate_or);
    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("PUT")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::NonCacheable(_)));
}

#[test]
fn test_request_predicate_query_in_serialize() {
    let query_params = vec![("cache".to_owned(), vec!["true".to_owned(), "1".to_owned()])]
        .into_iter()
        .collect();
    let method = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let path = Expression::Predicate(Predicate::Path("/books".to_owned()));
    let query = Expression::Predicate(Predicate::Query(QueryOperation::In(query_params)));
    let and_ = Expression::Operation(Operation::And(vec![method.into(), path.into()]));
    let or_ = Expression::Operation(Operation::Or(vec![query.into(), and_.into()]));
    let request = Request::Tree(or_);
    let endpoint = Endpoint { request };
    let yaml_str = serde_yaml::to_string(&endpoint).unwrap();
    let expected = r"request:
  Or:
  - Query:
      operation: In
      cache:
      - 'true'
      - '1'
  - And:
    - Method: GET
    - Path: /books
";
    assert_eq!(yaml_str, expected);
}

#[test]
fn test_expression_flat_deserialize() {
    let yaml_str = r"request:
- Method: GET
- Path: /books
- Query:
    operation: Eq
    cache: 'true'
";
    let endpoint: Endpoint = serde_yaml::from_str(yaml_str).unwrap();

    let query_params = vec![("cache".to_owned(), "true".to_owned())]
        .into_iter()
        .collect();
    let method = Predicate::Method("GET".to_owned());
    let path = Predicate::Path("/books".to_owned());
    let query = Predicate::Query(QueryOperation::Eq(query_params));
    let request = Request::Flat(vec![method, path, query]);
    let expected = Endpoint { request };
    let yaml_str = serde_yaml::to_string(&endpoint).unwrap();
    println!("{}", yaml_str);
    assert_eq!(endpoint, expected);
}

#[tokio::test]
async fn test_or_with_matching_first_predicate() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let method_post = Expression::Predicate(Predicate::Method("POST".to_owned()));
    let method_head = Expression::Predicate(Predicate::Method("HEAD".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![
        method_get.into(),
        method_post.into(),
        method_head.into(),
    ]));
    let predicate_or = or_.into_predicates(inner);

    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("GET")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_or_with_matching_middle_predicate() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let method_post = Expression::Predicate(Predicate::Method("POST".to_owned()));
    let method_head = Expression::Predicate(Predicate::Method("HEAD".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![
        method_get.into(),
        method_post.into(),
        method_head.into(),
    ]));
    let predicate_or = or_.into_predicates(inner);

    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("POST")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_or_with_matching_last_predicate() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let method_post = Expression::Predicate(Predicate::Method("POST".to_owned()));
    let method_head = Expression::Predicate(Predicate::Method("HEAD".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![
        method_get.into(),
        method_post.into(),
        method_head.into(),
    ]));
    let predicate_or = or_.into_predicates(inner);

    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("HEAD")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_or_with_no_matching_predicates() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let method_post = Expression::Predicate(Predicate::Method("POST".to_owned()));
    let method_head = Expression::Predicate(Predicate::Method("HEAD".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![
        method_get.into(),
        method_post.into(),
        method_head.into(),
    ]));
    let predicate_or = or_.into_predicates(inner);

    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("DELETE")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_or_with_single_predicate_matching() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![method_get.into()]));
    let predicate_or = or_.into_predicates(inner);

    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("GET")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::Cacheable(_)));
}

#[tokio::test]
async fn test_or_with_single_predicate_not_matching() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_get = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![method_get.into()]));
    let predicate_or = or_.into_predicates(inner);

    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("POST")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::NonCacheable(_)));
}

#[tokio::test]
async fn test_or_with_mixed_predicate_types() {
    let inner = Box::new(NeutralRequestPredicate::new());
    let method_post = Expression::Predicate(Predicate::Method("POST".to_owned()));
    let path_books = Expression::Predicate(Predicate::Path("/books".to_owned()));
    let or_ = Expression::Operation(Operation::Or(vec![method_post.into(), path_books.into()]));
    let predicate_or = or_.into_predicates(inner);

    // Test request that matches the path but not the method
    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("GET")
            .uri("/books")
            .body(Empty::<Bytes>::new())
            .unwrap(),
    );
    let cacheable = predicate_or.check(request).await;
    assert!(matches!(cacheable, PredicateResult::Cacheable(_)));
}
