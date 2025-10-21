Feature: Logical And Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```

  @integration
  Scenario: And predicate - both predicates match - request cached
    Given request predicates
      ```yaml
      And:
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
  Scenario: And predicate - first predicate matches, second doesn't - request not cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Path: /v1/different-path
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - first predicate doesn't match - short-circuit - request not cached
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Path: /v1/authors/robert-sheckley/books/victim-prime
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - both predicates don't match - request not cached
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Path: /v1/different-path
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
