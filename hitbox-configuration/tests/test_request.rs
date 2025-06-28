use hitbox_configuration::{
    Endpoint,
    predicates::request::{Expression, Operation, Predicate, QueryOperation, Request},
};

#[test]
fn test_expression_tree_serialize() {
    let method = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let path = Expression::Predicate(Predicate::Path("/books".to_owned()));
    let query = Expression::Predicate(Predicate::Query(QueryOperation::Eq {
        name: "cache".to_owned(),
        value: "true".to_owned(),
    }));
    let and_ = Expression::Operation(Operation::And(method.into(), path.into()));
    let or_ = Expression::Operation(Operation::Or(query.into(), and_.into()));
    let request = Request::Tree(or_);
    let endpoint = Endpoint { request };
    let yaml_str = serde_yaml::to_string(&endpoint).unwrap();
    let expected = r"request:
  Or:
  - Query:
      name: cache
      value: 'true'
  - And:
    - Method: GET
    - Path: /books
";
    assert_eq!(yaml_str, expected);
}

#[test]
fn test_expression_tree_deserialize() {
    let yaml_str = r"request:
  Or:
  - Query:
      name: cache
      value: 'true'
  - And:
    - Method: GET
    - Path: /books
";
    let endpoint: Endpoint = serde_yaml::from_str(yaml_str).unwrap();

    let method = Expression::Predicate(Predicate::Method("GET".to_owned()));
    let path = Expression::Predicate(Predicate::Path("/books".to_owned()));
    let query = Expression::Predicate(Predicate::Query(QueryOperation::Eq {
        name: "cache".to_owned(),
        value: "true".to_owned(),
    }));
    let and_ = Expression::Operation(Operation::And(method.into(), path.into()));
    let or_ = Expression::Operation(Operation::Or(query.into(), and_.into()));
    let request = Request::Tree(or_);
    let expected = Endpoint { request };

    assert_eq!(endpoint, expected);
}

#[test]
fn test_expression_flat_serialize() {
    let method = Predicate::Method("GET".to_owned());
    let path = Predicate::Path("/books".to_owned());
    let query = Predicate::Query(QueryOperation::Eq {
        name: "cache".to_owned(),
        value: "true".to_owned(),
    });
    let request = Request::Flat(vec![method, path, query]);
    let endpoint = Endpoint { request };
    let yaml_str = serde_yaml::to_string(&endpoint).unwrap();
    println!("{}", yaml_str);
    let expected = r"request:
- Method: GET
- Path: /books
- Query:
    name: cache
    value: 'true'
";
    assert_eq!(yaml_str, expected);
}

#[test]
fn test_expression_flat_deserialize() {
    let yaml_str = r"request:
- Method: GET
- Path: /books
- Query:
    name: cache
    value: 'true'
";
    let endpoint: Endpoint = serde_yaml::from_str(yaml_str).unwrap();

    let method = Predicate::Method("GET".to_owned());
    let path = Predicate::Path("/books".to_owned());
    let query = Predicate::Query(QueryOperation::Eq {
        name: "cache".to_owned(),
        value: "true".to_owned(),
    });
    let request = Request::Flat(vec![method, path, query]);
    let expected = Endpoint { request };
    let yaml_str = serde_yaml::to_string(&endpoint).unwrap();
    println!("{}", yaml_str);
    assert_eq!(endpoint, expected);
}
