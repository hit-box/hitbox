Feature: Request Query Cache Key Extractor

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Extract query parameter for cache key
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Query: "page"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      ```
    Then cache key "page=1" exists

  @integration
  Scenario: Missing query parameter creates cache key without that part
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Method:
      - Query: "page"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      ```
    Then cache key "method=GET" exists

  @integration
  Scenario: Multiple query parameters
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Query: "page"
      - Query: "limit"
      - Query: "sort"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 2
      limit: 20
      sort: title
      ```
    Then cache key "page=2:limit=20:sort=title" exists
