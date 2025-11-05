Feature: Response Header Regex Predicate

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Header Regex - header value matches regex pattern - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            regex: "application/(json|xml)"
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
  Scenario: Header Regex - header value doesn't match pattern - response not cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            regex: "text/.*"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Regex - complex regex patterns - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            regex: "^application/json.*"
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
  Scenario: Header Regex - multiple headers with regex - response cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            regex: "application/.*"
          server:
            regex: "hitbox-.*"
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
  Scenario: Header Regex - header missing - response not cached
    Given response predicates
      ```yaml
      - Header:
          x-custom-header:
            regex: ".*"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Regex - case-sensitive regex matching - response not cached
    Given response predicates
      ```yaml
      - Header:
          content-type:
            regex: "application/JSON"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
