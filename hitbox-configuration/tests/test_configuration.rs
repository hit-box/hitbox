use hitbox_configuration::{
    Endpoint, Expression, HeaderOperation, Predicate, QueryOperation, Request,
};

#[test]
fn test_expression_tree() {
    let header = Predicate::Header(HeaderOperation::Eq {
        name: "X-Cache".to_owned(),
        value: "true".to_owned(),
    });
    let query = Predicate::Query(QueryOperation::Eq {
        name: "cache".to_owned(),
        value: "true".to_owned(),
    });
    let method = Predicate::Method("GET".to_owned());

    let expression = Expression::And(
        Box::new(Expression::Or(
            Box::new(Expression::Predicate(header)),
            Box::new(Expression::Predicate(query)),
        )),
        Box::new(Expression::Predicate(method)),
    );

    let endpoint = Endpoint { request: Request::Tree(expression) };
    let rep = serde_yaml::to_string(&endpoint).unwrap();
    println!("{}", &rep);
    let endpoint : Endpoint = serde_yaml::from_str(rep.as_str()).unwrap();
    dbg!(endpoint);
    unreachable!()
}
