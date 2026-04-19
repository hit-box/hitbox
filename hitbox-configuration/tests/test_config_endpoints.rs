use bytes::Bytes;
use hitbox_configuration::{ConfigEndpoint, ConfigEndpoints, ConfigError, Endpoint};
use http_body_util::Empty;

type ReqBody = Empty<Bytes>;
type ResBody = Empty<Bytes>;

#[test]
fn test_single_endpoint_with_name() {
    let yaml_input = r"
name: my-endpoint
request:
- Method: GET
policy:
  Enabled:
    ttl: 60s
";
    let config: ConfigEndpoint = serde_saphyr::from_str(yaml_input).unwrap();
    assert_eq!(config.name, Some("my-endpoint".to_string()));
}

#[test]
fn test_single_endpoint_without_name() {
    let yaml_input = r"
request:
- Method: GET
policy:
  Enabled:
    ttl: 60s
";
    let config: ConfigEndpoint = serde_saphyr::from_str(yaml_input).unwrap();
    assert_eq!(config.name, None);
}

#[test]
fn test_parse_multi_endpoint_list() {
    let yaml_input = r"
- name: users
  request:
    - Method: GET
    - Path: /api/users/{id}
  response:
    - Status: 200
  extractors:
    - Method: ~
    - Path: '/api/users/{id}'
  policy:
    Enabled:
      ttl: 300s

- name: orders
  request:
    - Method: POST
    - Path: /api/orders
  policy:
    Enabled:
      ttl: 60s

- request:
    - Method: GET
  policy:
    Enabled:
      ttl: 30s
";
    let config: ConfigEndpoints = serde_saphyr::from_str(yaml_input).unwrap();
    assert_eq!(config.len(), 3);
    assert_eq!(config.endpoints[0].name, Some("users".to_string()));
    assert_eq!(config.endpoints[1].name, Some("orders".to_string()));
    assert_eq!(config.endpoints[2].name, None);
}

#[test]
fn test_into_endpoints_count() {
    let yaml_input = r"
- request:
    - Method: GET
  policy:
    Enabled:
      ttl: 60s
- request:
    - Method: POST
  policy:
    Enabled:
      ttl: 120s
";
    let config: ConfigEndpoints = serde_saphyr::from_str(yaml_input).unwrap();
    let endpoints = config.into_endpoints::<ReqBody, ResBody>().unwrap();
    assert_eq!(endpoints.len(), 2);
}

#[test]
fn test_into_endpoints_order_preservation() {
    let yaml_input = r"
- policy:
    Enabled:
      ttl: 10s
- policy:
    Enabled:
      ttl: 20s
- policy:
    Enabled:
      ttl: 30s
";
    let config: ConfigEndpoints = serde_saphyr::from_str(yaml_input).unwrap();
    let endpoints = config.into_endpoints::<ReqBody, ResBody>().unwrap();
    assert_eq!(endpoints.len(), 3);

    // Verify order matches YAML order via policy TTL
    use hitbox::policy::PolicyConfig;
    use std::time::Duration;
    for (i, expected_secs) in [10, 20, 30].iter().enumerate() {
        match endpoints[i].policy.as_ref() {
            PolicyConfig::Enabled(cfg) => {
                assert_eq!(cfg.ttl, Some(Duration::from_secs(*expected_secs)));
            }
            PolicyConfig::Disabled => panic!("Expected Enabled policy for endpoint {}", i),
        }
    }
}

#[test]
fn test_single_element_list() {
    let yaml_input = r"
- request:
    - Method: GET
  policy:
    Enabled:
      ttl: 60s
";
    let config: ConfigEndpoints = serde_saphyr::from_str(yaml_input).unwrap();
    let endpoints = config.into_endpoints::<ReqBody, ResBody>().unwrap();
    assert_eq!(endpoints.len(), 1);
}

#[test]
fn test_name_propagates_to_endpoint() {
    let yaml_input = r"
- name: my-cached-endpoint
  request:
    - Method: GET
  policy:
    Enabled:
      ttl: 60s
";
    let config: ConfigEndpoints = serde_saphyr::from_str(yaml_input).unwrap();
    let endpoints = config.into_endpoints::<ReqBody, ResBody>().unwrap();
    assert_eq!(endpoints[0].name, Some("my-cached-endpoint".to_string()));
}

#[test]
fn test_empty_list() {
    let yaml_input = "[]";
    let config: ConfigEndpoints = serde_saphyr::from_str(yaml_input).unwrap();
    let endpoints = config.into_endpoints::<ReqBody, ResBody>().unwrap();
    assert!(endpoints.is_empty());
}

#[test]
fn test_endpoint_builder_sets_name() {
    let endpoint: Endpoint<ReqBody, ResBody> = Endpoint::builder().name("custom-name").build();
    assert_eq!(endpoint.name, Some("custom-name".to_string()));
}

#[test]
fn test_endpoint_builder_without_name_defaults_to_none() {
    let endpoint: Endpoint<ReqBody, ResBody> = Endpoint::builder().build();
    assert_eq!(endpoint.name, None);
}

#[test]
fn test_into_endpoints_error_includes_index_and_name() {
    // Second entry has a malformed HTTP method that fails during conversion.
    let yaml_input = r#"
- name: good-endpoint
  request:
    - Method: GET
  policy:
    Enabled:
      ttl: 60s
- name: broken-endpoint
  request:
    - Method: "INVALID METHOD"
  policy:
    Enabled:
      ttl: 60s
"#;
    let config: ConfigEndpoints = serde_saphyr::from_str(yaml_input).unwrap();
    let err = config
        .into_endpoints::<ReqBody, ResBody>()
        .expect_err("conversion should fail on invalid HTTP method");

    match err {
        ConfigError::EndpointAt {
            index,
            name,
            source,
        } => {
            assert_eq!(index, 1);
            assert_eq!(name, Some("broken-endpoint".to_string()));
            assert!(matches!(*source, ConfigError::InvalidMethod(..)));
        }
        other => panic!("expected ConfigError::EndpointAt, got: {other:?}"),
    }
}
