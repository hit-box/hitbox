Feature: Multipul request preducates

  Background:
    Given hitbox with policy
    ```yaml
      Enabled:
        ttl: 10
    ```

  Scenario: Four request predicates chained
    Given request predicates
    ```yaml
        - Method: GET
        - Path: /v1/authors/robert-sheckley/books/{id}
        - Header:
            User-Agent: googlebot
        - Query:
            page: 10
    ```
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
    User-Agent: googlebot
    [Query]
    page: 10
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
    User-Agent: googlebot
    [Query]
    page: 10
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

