// @FIX: assert with final `crate::Endpoint`
use bytes::Bytes;
use hitbox_configuration::{
    ConfigEndpoint, Response,
    predicates::response::{Predicate, status},
    types::MaybeUndefined,
};
use http_body_util::Empty;
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
