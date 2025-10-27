Feature: Logical Not Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Not predicate - wrapped predicate matches - result is NonCacheable
    Given request predicates
      ```yaml
      Not:
        Method: POST
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Not predicate - wrapped predicate doesn't match - result is Cacheable
    Given request predicates
      ```yaml
      Not:
        Method: POST
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
