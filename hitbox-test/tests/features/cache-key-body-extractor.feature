Feature: Request Body Cache Key Extractor

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Extract JSON field from request body for cache key
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body: '.title'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"title":"Test Book","description":"Test Description"}
      ```
    Then cache key ".title='Test Book'" exists
