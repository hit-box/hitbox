Feature: Cache Key Extractor Combinations

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Method + Path extractors combined
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then cache key exists
      ```
      method: "GET"
      author_id: "robert-sheckley"
      book_id: "victim-prime"
      ```

  @integration
  Scenario: Method + Query extractors combined
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Method:
      - Query: "page"
      - Query: "limit"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      limit: 10
      ```
    Then cache key exists
      ```
      method: "GET"
      page: "1"
      limit: "10"
      ```

  @integration
  Scenario: Header + Body extractors combined
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Header: X-Tenant-Id
      - Body: '.userId'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-tenant-id: tenant-123
      {"userId":"user-456","action":"update"}
      ```
    Then cache key exists
      ```
      X-Tenant-Id: "tenant-123"
      .userId: "user-456"
      ```

  @integration
  Scenario: All extractors combined (Method + Path + Query + Header + Body)
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      - Query: "includeDeleted"
      - Header: X-Api-Version
      - Body: '.role'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-version: v2
      [Query]
      includeDeleted: false
      {"role":"admin","name":"John Doe"}
      ```
    Then cache key exists
      ```
      method: "GET"
      author_id: "robert-sheckley"
      book_id: "victim-prime"
      includeDeleted: "false"
      X-Api-Version: "v2"
      .role: "admin"
      ```
