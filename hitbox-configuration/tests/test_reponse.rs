// @FIX: assert with final `crate::Endpoint`
use bytes::Bytes;
use hitbox_configuration::{
    ConfigEndpoint, Response,
    predicates::response::{Predicate, status, header},
    types::MaybeUndefined,
};
use http_body_util::Empty;
use indexmap::IndexMap;
use pretty_assertions::assert_eq;
use std::num::NonZeroU16;

#[test]
fn test_response_expression_flat_deserialize() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
response:
  - Status: 200
  - Status: 201
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();
    let expected = ConfigEndpoint {
        response: MaybeUndefined::Value(Response::Flat(vec![
            Predicate::Status(status::Operation::Eq(status::Eq::Implicit(NonZeroU16::new(200).unwrap()))),
            Predicate::Status(status::Operation::Eq(status::Eq::Implicit(NonZeroU16::new(201).unwrap()))),
        ])),
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}

#[test]
fn test_undefined_response_predicates() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();
    let expected = ConfigEndpoint {
        response: MaybeUndefined::Undefined,
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}

#[test]
fn test_null_response_predicates() {
    let yaml_str = r"
response: null
policy:
  Enabled:
    ttl: 5
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();
    let expected = ConfigEndpoint {
        response: MaybeUndefined::Null,
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}

#[test]
fn test_response_expression_into_predicates() {
    let yaml_str = r"
extractors: []
policy:
  Enabled:
    ttl: 5
response:
  And:
  - Status: 203
  - Or:
    - Status: 201
    - Status: 200
    - Status: 202
  - Status: 205
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();
    dbg!(&endpoint.response);
    let predicates = endpoint
        .response
        .unwrap_or_default()
        .into_predicates::<Empty<Bytes>>();
    dbg!(predicates);
    // let expected = Endpoint {
    //     response: Response::Flat(vec![
    //         Predicate::Status(200.try_into().unwrap()),
    //         Predicate::Status(201.try_into().unwrap()),
    //     ]),
    //     ..Default::default()
    // };
    // assert_eq!(endpoint, expected);
}

#[test]
fn test_invalid_status_range_rejected() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
response:
  - Status:
      range: [299, 200]
";
    let result = serde_saphyr::from_str::<ConfigEndpoint>(yaml_str);
    assert!(
        result.is_err(),
        "Invalid range (start > end) should be rejected during deserialization"
    );
}

#[test]
fn test_valid_status_range_accepted() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
response:
  - Status:
      range: [200, 299]
";
    let result = serde_saphyr::from_str::<ConfigEndpoint>(yaml_str);
    assert!(result.is_ok(), "Valid range should be accepted: {:?}", result.err());
}

#[test]
fn test_response_header_eq_deserialize() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
response:
  - Header:
      content-type: application/json
      cache-control: max-age=3600
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();

    let mut expected_headers = IndexMap::new();
    expected_headers.insert("content-type".to_string(), "application/json".to_string());
    expected_headers.insert("cache-control".to_string(), "max-age=3600".to_string());

    let expected = ConfigEndpoint {
        response: MaybeUndefined::Value(Response::Flat(vec![
            Predicate::Header(header::HeaderOperation::Eq(expected_headers)),
        ])),
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}

#[test]
fn test_response_header_exist_deserialize() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
response:
  - Header: x-custom-header
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();
    let expected = ConfigEndpoint {
        response: MaybeUndefined::Value(Response::Flat(vec![
            Predicate::Header(header::HeaderOperation::Exist("x-custom-header".to_string())),
        ])),
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}

#[test]
fn test_response_header_in_deserialize() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
response:
  - Header:
      content-type:
        - application/json
        - application/xml
      accept:
        - text/html
        - text/plain
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();

    let mut expected_headers = IndexMap::new();
    expected_headers.insert(
        "content-type".to_string(),
        vec!["application/json".to_string(), "application/xml".to_string()]
    );
    expected_headers.insert(
        "accept".to_string(),
        vec!["text/html".to_string(), "text/plain".to_string()]
    );

    let expected = ConfigEndpoint {
        response: MaybeUndefined::Value(Response::Flat(vec![
            Predicate::Header(header::HeaderOperation::In(expected_headers)),
        ])),
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}

#[test]
fn test_response_header_combined_with_status() {
    let yaml_str = r"
policy:
  Enabled:
    ttl: 5
response:
  - Status: 200
  - Header:
      content-type: application/json
";
    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml_str).unwrap();

    let mut expected_headers = IndexMap::new();
    expected_headers.insert("content-type".to_string(), "application/json".to_string());

    let expected = ConfigEndpoint {
        response: MaybeUndefined::Value(Response::Flat(vec![
            Predicate::Status(status::Operation::Eq(status::Eq::Implicit(NonZeroU16::new(200).unwrap()))),
            Predicate::Header(header::HeaderOperation::Eq(expected_headers)),
        ])),
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}
