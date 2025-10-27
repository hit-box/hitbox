Feature: Logical Or Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
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

  @integration
  Scenario: Or predicate - three predicates in Or - first matches
    Given request predicates
      ```yaml
      Or:
        - Method: GET
        - Method: HEAD
        - Method: OPTIONS
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
  Scenario: Or predicate - three predicates in Or - second matches
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Method: GET
        - Method: DELETE
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
  Scenario: Or predicate - three predicates in Or - third matches
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Method: DELETE
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
  Scenario: Or predicate - three predicates in Or - none match
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Method: DELETE
        - Method: PUT
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Or predicate - different predicate types - Method OR Path OR Header
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Path: /v1/authors/robert-sheckley/books/victim-prime
        - Header:
            x-bypass-cache: "true"
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
  Scenario: Or predicate - different types all fail - Method OR Path OR Header - not cached
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Path: /v1/authors/robert-sheckley/books/victim-prime
        - Header:
            x-bypass-cache: "true"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/immortality-inc
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Or predicate - Method OR Header OR Query - all fail - not cached
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Header:
            x-admin: "true"
        - Query:
            cache: "force"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Or predicate - four predicates - first matches - cached
    Given request predicates
      ```yaml
      Or:
        - Method: GET
        - Method: POST
        - Method: PUT
        - Method: DELETE
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
  Scenario: Or predicate - four predicates - all fail - not cached
    Given request predicates
      ```yaml
      Or:
        - Method: POST
        - Method: PUT
        - Method: PATCH
        - Method: DELETE
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
