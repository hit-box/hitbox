use bytes::Bytes;
use hitbox::policy::PolicyConfig;
use hitbox_configuration::{ConfigEndpoint, types::MaybeUndefined};
use http_body_util::Empty;
use std::time::Duration;

type ReqBody = Empty<Bytes>;
type ResBody = Empty<Bytes>;

fn default_policy() -> PolicyConfig {
    PolicyConfig::builder().ttl(Duration::from_secs(60)).build()
}

#[test]
fn test_extractors_variants() {
    for extractors in [
        MaybeUndefined::Undefined,
        MaybeUndefined::Null,
        MaybeUndefined::Value(vec![]),
    ] {
        let endpoint = ConfigEndpoint {
            extractors,
            request: MaybeUndefined::Undefined,
            response: MaybeUndefined::Undefined,
            policy: default_policy(),
        };
        assert!(endpoint.extractors::<ReqBody>().is_ok());
    }
}

#[test]
fn test_into_endpoint_variants() {
    // all undefined
    let endpoint = ConfigEndpoint {
        extractors: MaybeUndefined::Undefined,
        request: MaybeUndefined::Undefined,
        response: MaybeUndefined::Undefined,
        policy: default_policy(),
    };
    assert!(endpoint.into_endpoint::<ReqBody, ResBody>().is_ok());

    // all null
    let endpoint = ConfigEndpoint {
        extractors: MaybeUndefined::Null,
        request: MaybeUndefined::Null,
        response: MaybeUndefined::Null,
        policy: default_policy(),
    };
    assert!(endpoint.into_endpoint::<ReqBody, ResBody>().is_ok());

    // default
    let endpoint = ConfigEndpoint::default();
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
