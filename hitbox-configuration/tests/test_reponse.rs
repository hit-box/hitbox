use hitbox_configuration::{Endpoint, Response, predicates::response::Predicate};
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
