use hitbox::predicates::{Operation, Predicate};
use hitbox_http::predicates::Header;
use http::Request;
use hyper::Body;

#[test]
fn test_request_header_predicates() {
    let request = Request::builder()
        .header("x-test", "test-value")
        .body(Body::empty())
        .unwrap();
    let predicate = Header {
        name: "x-test".to_owned(),
        value: "test-value".to_owned(),
        operation: Operation::Eq,
    };
    assert!(predicate.check(&request));

    let inapplicable_predicate = Header {
        name: "x-test2".to_owned(),
        value: "test-value".to_owned(),
        operation: Operation::Eq,
    };
    assert!(!inapplicable_predicate.check(&request));

    let inapplicable_predicate = Header {
        name: "x-test".to_owned(),
        value: "test-value2".to_owned(),
        operation: Operation::Eq,
    };
    assert!(!inapplicable_predicate.check(&request));
}
