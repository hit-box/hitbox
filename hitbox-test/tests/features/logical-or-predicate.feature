Feature: Logical Or Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```

  @integration
  Scenario: Or predicate - left predicate matches - request cached
    Given request predicates
      ```yaml
      Or:
        - Method: GET
        - Method: POST
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
  Scenario: Or predicate - right predicate matches - request cached
    Given request predicates
      ```yaml
      Or:
        - Method: POST
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
  Scenario: Or predicate - both predicates match - request cached
    Given request predicates
      ```yaml
      Or:
        - Method: GET
        - Path: /v1/authors/robert-sheckley/books/victim-prime
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
  Scenario: Or predicate - neither predicate matches - request not cached
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Method: DELETE
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Or predicate - base predicate fails - request not cached regardless of Or branches
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Or:
          - Path: /v1/authors/robert-sheckley/books/victim-prime
          - Header:
              x-api-key: "secret123"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: secret123
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
