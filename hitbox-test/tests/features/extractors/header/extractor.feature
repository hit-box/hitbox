Feature: Request Header Cache Key Extractor

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10s
      ```

  @extractor @header
  Scenario: Extract header value for cache key
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header: X-Api-Key
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: secret123
      ```
    Then cache key exists
      ```
      X-Api-Key: "secret123"
      ```

  @extractor @header
  Scenario: Missing header creates cache key without that part
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Method:
      - Header: X-Tenant-Id
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then cache key exists
      ```
      method: "GET"
      X-Tenant-Id: null
      ```

  @extractor @header
  Scenario: Multiple header extractors
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header: X-Tenant-Id
      - Header: X-User-Id
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-tenant-id: tenant-123
      x-user-id: user-456
      ```
    Then cache key exists
      ```
      X-Tenant-Id: "tenant-123"
      X-User-Id: "user-456"
      ```

  @extractor @header @regex
  Scenario: Extract header value with regex - Bearer token
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name: Authorization
          value: "Bearer (.+)"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Authorization: Bearer my-secret-token-123
      ```
    Then cache key exists
      ```
      Authorization: "my-secret-token-123"
      ```

  @extractor @header @hash
  Scenario: Extract header value with hash transform
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name: Authorization
          transforms: [hash]
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Authorization: Bearer my-secret-token-123
      ```
    Then cache key exists
      ```
      Authorization: "f5603935428bbcd5d789830e83553f4b06b5187c3c91145a777920248f20ed47"
      ```

  @extractor @header @regex @hash
  Scenario: Extract Bearer token with regex and hash
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name: Authorization
          value: "Bearer (.+)"
          transforms: [hash]
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Authorization: Bearer my-secret-token-123
      ```
    Then cache key exists
      ```
      Authorization: "3e5358294ba0321e7b4a6acd192104f2e9c9776e8645bc24d4332e266596cc72"
      ```

  @extractor @header @starts
  Scenario: Extract headers by prefix
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name:
            starts: x-custom-
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-custom-foo: value1
      x-custom-bar: value2
      x-other-header: ignored
      ```
    Then cache key exists
      ```
      x-custom-bar: "value2"
      x-custom-foo: "value1"
      ```

  @extractor @header @explicit-eq
  Scenario: Extract header with explicit eq operation
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name:
            eq: X-Request-Id
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      X-Request-Id: req-12345
      ```
    Then cache key exists
      ```
      X-Request-Id: "req-12345"
      ```

  @extractor @header @transforms
  Scenario: Extract header value with transform chain
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name: X-User-Email
          transforms: [lowercase, hash]
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      X-User-Email: User@Example.COM
      ```
    Then cache key exists
      | X-User-Email | b4c9a289323b21a01c3e940f150eb9b8c542587f1abfd8f0e1cc1ffc5e475514 |

  @extractor @header @transforms
  Scenario: Extract header value with lowercase transform
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name: X-Status
          transforms: [lowercase]
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      X-Status: ACTIVE
      ```
    Then cache key exists
      | X-Status | active |

  @extractor @header @transforms
  Scenario: Extract header value with uppercase transform
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header:
          name: X-Code
          transforms: [uppercase]
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      X-Code: abc123
      ```
    Then cache key exists
      | X-Code | ABC123 |
