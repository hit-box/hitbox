Feature: Response Header Exist Predicate

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Header Exist - header exists - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            exist: true
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
  Scenario: Header Exist - header missing - response not cached
    Given response predicates
      ```yaml
      - Header:
          x-custom-header:
            exist: true
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Exist - case-insensitive header name - response cached
    Given response predicates
      ```yaml
      - Header:
          Content-TYPE:
            exist: true
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
  Scenario: Header Exist - additional header doesn't affect cache decision - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            exist: true
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
      [Query]
      test_headers: true
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Exist - header with any value matches - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            exist: true
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
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
