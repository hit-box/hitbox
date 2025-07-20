use bytes::Bytes;
use hitbox_configuration::{Endpoint, Response, predicates::response::Predicate};
use http_body_util::Empty;
use pretty_assertions::assert_eq;

#[test]
fn test_response_expression_flat_deserialize() {
    let yaml_str = r"
request: []
extractors: []
policy: !Enabled
  ttl: 5
response:
  - Status: 200
  - Status: 201
";
    let endpoint: Endpoint = serde_yaml::from_str(yaml_str).unwrap();
    let expected = Endpoint {
        response: Response::Flat(vec![
            Predicate::Status(200.try_into().unwrap()),
            Predicate::Status(201.try_into().unwrap()),
        ]),
        ..Default::default()
    };
    assert_eq!(endpoint, expected);
}

#[test]
fn test_response_expression_into_predicates() {
    let yaml_str = r"
request: []
extractors: []
policy: !Enabled
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
    let endpoint: Endpoint = serde_yaml::from_str(yaml_str).unwrap();
    dbg!(&endpoint.response);
    let predicates = endpoint.response.into_predicates::<Empty<Bytes>>();
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
