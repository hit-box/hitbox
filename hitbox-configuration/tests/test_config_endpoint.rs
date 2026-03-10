use bytes::Bytes;
use hitbox_configuration::{ConfigEndpoint, types::MaybeUndefined};
use http_body_util::Empty;

type ReqBody = Empty<Bytes>;
type ResBody = Empty<Bytes>;

#[test]
fn test_extractors_variants() {
    for extractors in [
        MaybeUndefined::Undefined,
        MaybeUndefined::Null,
        MaybeUndefined::Value(vec![]),
    ] {
        let endpoint = ConfigEndpoint {
            extractors,
            ..Default::default()
        };
        assert!(endpoint.extractors::<ReqBody>().is_ok());
    }
}

#[test]
fn test_into_endpoint_variants() {
    // all undefined
    let endpoint = ConfigEndpoint::default();
    assert!(endpoint.into_endpoint::<ReqBody, ResBody>().is_ok());

    // all null
    let endpoint = ConfigEndpoint {
        extractors: MaybeUndefined::Null,
        request: MaybeUndefined::Null,
        response: MaybeUndefined::Null,
        ..Default::default()
    };
    assert!(endpoint.into_endpoint::<ReqBody, ResBody>().is_ok());
}

#[test]
fn test_into_endpoint_from_yaml() {
    let yaml_input = r"
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
    ttl: 60s
";
    let config: ConfigEndpoint = serde_saphyr::from_str(yaml_input).unwrap();
    assert!(config.into_endpoint::<ReqBody, ResBody>().is_ok());
}
