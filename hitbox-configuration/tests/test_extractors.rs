use hitbox_configuration::{
    ConfigEndpoint,
    extractors::{Extractor, method::Method, path::Path, version::Version},
    types::MaybeUndefined,
};
use pretty_assertions::assert_eq;

#[test]
fn test_extractors_serialize() {
    let extractors = vec![
        Extractor::Method(Method::new()),
        Extractor::Path(Path::new("/greet/{name}")),
    ];
    let original_endpoint = ConfigEndpoint {
        extractors: MaybeUndefined::Value(extractors),
        ..Default::default()
    };
    let yaml_str = serde_saphyr::to_string(&original_endpoint).unwrap();
    println!("{}", yaml_str);
    let config = r"
policy:
  Enabled: {}
extractors:
  - Method: {}
  - Path: /greet/{name}
";
    let endpoint = serde_saphyr::from_str(config).unwrap();
    dbg!(&endpoint);
    assert_eq!(original_endpoint, endpoint);
}

#[test]
fn test_extractors_without_options_accept_null_and_empty_mapping() {
    // An empty value (`- Method:`) is YAML null, which serde-saphyr 2.1 rejects for braced
    // structs; `Method` and `Version` are unit structs so the documented form keeps parsing.
    let config = r"
- Method:
- Method: {}
- Version:
- Version: {}
";
    let extractors: Vec<Extractor> = serde_saphyr::from_str(config).unwrap();
    assert_eq!(
        extractors,
        vec![
            Extractor::Method(Method::new()),
            Extractor::Method(Method::new()),
            Extractor::Version(Version::new()),
            Extractor::Version(Version::new()),
        ]
    );
}
