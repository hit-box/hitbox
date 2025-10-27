Feature: Request Path Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Path Eq operation - exact path match cached
    Given request predicates
      ```yaml
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
  Scenario: Path Eq operation - partial path match not cached
    Given request predicates
      ```yaml
      - Path: /v1/authors/robert-sheckley/books/victim-prime
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Path Eq operation - path with path parameter cached
    Given request predicates
      ```yaml
      - Path: /v1/authors/{author_id}/books/{book_id}
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
  Scenario: Path Eq operation - path with trailing slash doesn't match path without trailing slash
    Given request predicates
      ```yaml
      - Path: /v1/authors/robert-sheckley/books/victim-prime/
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Path Eq operation - path case sensitivity
    Given request predicates
      ```yaml
      - Path: /v1/authors/robert-sheckley/books/victim-prime
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/Robert-Sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Path Eq operation - path with encoded characters
    Given request predicates
      ```yaml
      - Path: /v1/authors/{author_id}/books/{book_id}
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert%20sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert%20sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And cache has 1 records
