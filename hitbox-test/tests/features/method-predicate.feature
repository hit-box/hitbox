Feature: Method Predicate Functionality

  @integration
  Scenario: Method predicate matches GET requests
    Given hitbox with policy
    ```yaml
    !Enabled
    ttl: 10
    ```
    And request predicates
    ```yaml
    - Method: GET
    ```
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Method predicate matches HEAD requests
    Given hitbox with policy
    ```yaml
    !Enabled
    ttl: 10
    ```
    And request predicates
    ```yaml
    - Method: HEAD
    ```
    When execute request
    ```hurl
    HEAD http://localhost/v1/authors/robert-sheckley/books/victim-prime
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Method In operation - GET in allowed list cached
    Given hitbox with policy
    ```yaml
    !Enabled
    ttl: 10
    ```
    And request predicates
    ```yaml
    - Method:
        - GET
        - HEAD
    ```
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
    ```hurl
    GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Method In operation - HEAD in allowed list cached
    Given hitbox with policy
    ```yaml
    !Enabled
    ttl: 10
    ```
    And request predicates
    ```yaml
    - Method:
        - GET
        - HEAD
    ```
    When execute request
    ```hurl
    HEAD http://localhost/v1/authors/robert-sheckley/books/victim-prime
    ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Method In operation - POST not in allowed list not cached
    Given hitbox with policy
    ```yaml
    !Enabled
    ttl: 10
    ```
    And request predicates
    ```yaml
    - Method:
        - GET
        - HEAD
    ```
    When execute request
    ```hurl
    POST http://localhost/v1/authors/robert-sheckley/books/victim-prime
    ```
    Then response status is 405
    And cache has 0 records
