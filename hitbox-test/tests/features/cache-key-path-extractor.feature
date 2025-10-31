Feature: Request Path Cache Key Extractor

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Extract path parameters for cache key
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then cache key exists
      """
      {"parts":[{"key":"author_id","value":"robert-sheckley"},{"key":"book_id","value":"victim-prime"}],"version":0,"prefix":""}
      """

  @integration
  Scenario: Path with no parameters extracts nothing
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Path: "v1/authors/robert-sheckley/books/victim-prime"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then cache key exists
      """
      {"parts":[],"version":0,"prefix":""}
      """
