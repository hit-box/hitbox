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
