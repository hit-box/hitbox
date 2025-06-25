use hitbox_configuration::{
    Condition, Endpoint, HeaderOperation, Predicate, QueryOperation, Request,
};

#[test]
fn test_serialize_flat() {
    let query = Predicate::Query(QueryOperation::Eq {
        name: "cache".to_owned(),
        value: "true".to_owned(),
    });
    let header = Predicate::Header(HeaderOperation::Eq {
        name: "X-Cache".to_owned(),
        value: "true".to_owned(),
    });
    let method = Predicate::Method("GET".to_owned());
    let endpoint = Endpoint {
        request: Request::Flat(vec![query, method, header]),
    };
    let serialized = serde_yaml::to_string(&endpoint).unwrap();
    print!("{}", serialized);
    let expected = r"request:
- Query:
    Eq:
      name: cache
      value: 'true'
- Method: GET
- Header:
    Eq:
      name: X-Cache
      value: 'true'
"
    .to_owned();
    assert_eq!(serialized, expected);
}

#[test]
fn test_serialize_recursive() {
    let header = Predicate::Header(HeaderOperation::Eq {
        name: "X-Cache".to_owned(),
        value: "true".to_owned(),
    });
    let method = Predicate::Method("GET".to_owned());
    let endpoint = Endpoint {
        request: Request::Recursive(Condition::And(method, header)),
    };

    let serialized = serde_yaml::to_string(&endpoint).unwrap();
    print!("{}", serialized);
    let expected = r"request:
  And:
  - Method: GET
  - Header:
      Eq:
        name: X-Cache
        value: 'true'
"
    .to_owned();
    assert_eq!(serialized, expected);
}

#[test]
fn test_serialize_mixed() {
    let query = Predicate::Query(QueryOperation::Eq {
        name: "cache".to_owned(),
        value: "true".to_owned(),
    });
    let header = Predicate::Header(HeaderOperation::Eq {
        name: "X-Cache".to_owned(),
        value: "true".to_owned(),
    });
    let method = Predicate::Method("GET".to_owned());
    let cond = Condition::Or(query, header);
    let endpoint = Endpoint {
        request: Request::Recursive(Condition::And(method, cond)),
    };
    let serialized = serde_yaml::to_string(&endpoint).unwrap();
    print!("{}", serialized);
    let expected = r"request:
- Query:
    Eq:
      name: cache
      value: 'true'
- Method: GET
- Header:
    Eq:
      name: X-Cache
      value: 'true'
"
    .to_owned();
    assert_eq!(serialized, expected);
}

#[test]
fn test_deserialize_flat() {
    let source_yaml = r"request:
- Query:
    Eq:
      name: cache
      value: 'true'
- Method: GET
- Header:
    Eq:
      name: X-Cache
      value: 'true'
"
    .to_owned();
    let endpoint = serde_yaml::from_str::<Endpoint>(&source_yaml);
    assert!(endpoint.is_ok())
}

#[test]
fn test_deserialize_recursive() {
    let source_yaml = r"request:
  And:
  - Method: GET
  - Header:
      Eq:
        name: X-Cache
        value: 'true'
"
    .to_owned();
    let endpoint = serde_yaml::from_str::<Endpoint>(&source_yaml);
    assert!(endpoint.is_ok())
}
