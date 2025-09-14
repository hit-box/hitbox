Feature: Query Parameter Cache Key Generation

  @integration
  Scenario: Different query parameters generate distinct cache keys
    Given hitbox with policy
    ```yaml
    !Enabled
    ttl: 3
    ```
    And key extractors
    ```yaml
    - Path: "/v1/authors/{author_id}/books"
    - Method:
    - Query: "page"
    - Query: "per_page"
    ```
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books
    [Query]
    page: 1
    per_page: 3
    ```
    Then response status is 200
    And response body jq 'length == 3'
    And cache has 1 records
    And response header "X-Cache-Status" is "MISS"
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books
    [Query]
    page: 2
    per_page: 3
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And response body jq 'length == 2'
    And cache has 2 records
