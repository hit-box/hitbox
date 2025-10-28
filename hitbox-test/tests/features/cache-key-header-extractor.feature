Feature: Request Header Cache Key Extractor

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
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
    Then cache key "X-Api-Key=secret123" exists

  @integration
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
    Then cache key "method=GET" exists

  @integration
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
    Then cache key "X-Tenant-Id=tenant-123:X-User-Id=user-456" exists
