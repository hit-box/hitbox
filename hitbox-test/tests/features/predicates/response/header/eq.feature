Feature: Response Header Eq Predicate

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Header Eq - exact header value match - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type: application/json
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
  Scenario: Header Eq - different header value - response not cached
    Given response predicates
      ```yaml
      - Header:
          content-type: text/html
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Eq - case-insensitive header name - response cached
    Given response predicates
      ```yaml
      - Header:
          CONTENT-Type: application/json
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
  Scenario: Header Eq - multiple header predicates all must match - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type: application/json
          server: hitbox-test
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Query]
      test_headers: true
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
  Scenario: Header Eq - header missing - response not cached
    Given response predicates
      ```yaml
      - Header:
          x-custom-header: some-value
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Eq - header with empty value - response cached
    Given response predicates
      ```yaml
      - Header:
          x-empty: ""
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Query]
      test_headers: true
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
  Scenario: Header Eq - multiple header values with EQ operation - response cached
    Given response predicates
      ```yaml
      - Header:
          set-cookie: session=abc123
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Query]
      test_headers: true
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
  Scenario: Header Eq - header value case sensitivity - response not cached
    Given response predicates
      ```yaml
      - Header:
          content-type: application/JSON
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Eq - header with whitespace trimmed - response cached
    Given response predicates
      ```yaml
      - Header:
          x-custom: "  value  "
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Query]
      test_headers: true
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
