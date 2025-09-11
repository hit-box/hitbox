use hitbox_configuration::{
    ConfigEndpoint,
    extractors::{Extractor, method::Method, path::Path},
};
use pretty_assertions::assert_eq;

#[test]
fn test_extractors_serialize() {
    let extractors = vec![
        Extractor::Method(Method::new()),
        Extractor::Path(Path::new("/greet/{name}")),
    ];
    let original_endpoint = ConfigEndpoint {
        extractors,
        ..Default::default()
    };
    let yaml_str = serde_yaml::to_string(&original_endpoint).unwrap();
    println!("{}", &yaml_str);
    let config = r"
    request: []
    response: null
    policy: !Enabled
      ttl: 5
    extractors:
      - Method:
      - Path: /greet/{name}
    ";
    let endpoint = serde_yaml::from_str(config).unwrap();
    dbg!(&endpoint);
    assert_eq!(original_endpoint, endpoint);
}
