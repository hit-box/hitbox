Feature: Request Header Regex Predicate

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Header Regex - header value matches regex pattern - request cached
    Given request predicates
      ```yaml
      - Header:
          Accept:
            regex: "application/(json|xml)"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Accept: application/json
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Accept: application/xml
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Regex - header value doesn't match pattern - request not cached
    Given request predicates
      ```yaml
      - Header:
          Accept:
            regex: "text/.*"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Accept: application/json
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Regex - complex regex patterns - request cached
    Given request predicates
      ```yaml
      - Header:
          User-Agent:
            regex: "^Mozilla/.*"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      User-Agent: Mozilla/5.0 (Windows NT 10.0)
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      User-Agent: Mozilla/5.0 (Macintosh)
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Regex - multiple headers with regex - request cached
    Given request predicates
      ```yaml
      - Header:
          Accept:
            regex: "application/.*"
          User-Agent:
            regex: "Mozilla/.*"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Accept: application/json
      User-Agent: Mozilla/5.0
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Accept: application/xml
      User-Agent: Mozilla/4.0
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Regex - header missing - request not cached
    Given request predicates
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
  Scenario: Header Regex - case-sensitive regex matching - request not cached
    Given request predicates
      ```yaml
      - Header:
          Accept:
            regex: "application/JSON"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Accept: application/json
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
