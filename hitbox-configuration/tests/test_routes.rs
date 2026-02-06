use bytes::Bytes;
use hitbox::{CacheConfigRouter, RouteMatch};
use hitbox_configuration::{ConfigEndpoint, ConfigError};
use hitbox_http::{BufferedBody, CacheableHttpRequest};
use http::Request as HttpRequest;
use http_body_util::Empty;
use std::time::Duration;

#[test]
fn test_multi_route_config_deserializes_and_builds() {
    let yaml = r"
routes:
  - request:
      - Method: GET
    response: []
    extractors: []
    policy:
      Enabled:
        ttl: 11s
  - request:
      - Method: POST
    response: []
    extractors: []
    policy:
      Enabled:
        ttl: 22s
";

    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml).unwrap();
    let routed = endpoint
        .into_routed_endpoint::<Empty<Bytes>, Empty<Bytes>>()
        .unwrap();

    assert_eq!(routed.len(), 2);
}

#[tokio::test]
async fn test_multi_route_first_match_selected() {
    let yaml = r"
routes:
  - request:
      - Method: GET
    response: []
    extractors: []
    policy:
      Enabled:
        ttl: 11s
  - request:
      - Method: GET
    response: []
    extractors: []
    policy:
      Enabled:
        ttl: 22s
";

    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml).unwrap();
    let routed = endpoint
        .into_routed_endpoint::<Empty<Bytes>, Empty<Bytes>>()
        .unwrap();

    let request = CacheableHttpRequest::from_request(
        HttpRequest::builder()
            .method("GET")
            .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
            .unwrap(),
    );

    match routed.route(request).await {
        RouteMatch::Matched { policy, .. } => {
            let expected = hitbox::policy::PolicyConfig::builder()
                .ttl(Duration::from_secs(11))
                .build();
            assert_eq!(policy, expected);
        }
        RouteMatch::Miss(_) => panic!("expected route match"),
    }
}

#[test]
fn test_into_endpoint_rejects_multiple_routes() {
    let yaml = r"
routes:
  - request:
      - Method: GET
    response: []
    extractors: []
    policy:
      Enabled:
        ttl: 11s
  - request:
      - Method: POST
    response: []
    extractors: []
    policy:
      Enabled:
        ttl: 22s
";

    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml).unwrap();
    let result = endpoint.into_endpoint::<Empty<Bytes>, Empty<Bytes>>();

    assert!(matches!(
        result,
        Err(ConfigError::MultipleRouteConfigurations(2))
    ));
}

#[test]
fn test_into_routed_endpoint_rejects_mixed_inline_and_routes() {
    let yaml = r"
request:
  - Method: GET
response: []
extractors: []
policy:
  Enabled:
    ttl: 5s
routes:
  - request:
      - Method: GET
    response: []
    extractors: []
    policy:
      Enabled:
        ttl: 11s
";

    let endpoint: ConfigEndpoint = serde_saphyr::from_str(yaml).unwrap();
    let result = endpoint.into_routed_endpoint::<Empty<Bytes>, Empty<Bytes>>();

    assert!(matches!(
        result,
        Err(ConfigError::MixedRouteAndInlineConfig)
    ));
}
