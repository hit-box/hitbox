Feature: Request Header Contains Predicate

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Header Contains - header value contains substring - request cached
    Given request predicates
      ```yaml
      - Header:
          User-Agent:
            contains: Mozilla
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64)
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Contains - header value doesn't contain substring - request not cached
    Given request predicates
      ```yaml
      - Header:
          User-Agent:
            contains: Chrome
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      User-Agent: Mozilla/5.0 Firefox/91.0
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Contains - case-sensitive substring matching - request not cached
    Given request predicates
      ```yaml
      - Header:
          x-api-key:
            contains: SECRET
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
  Scenario: Header Contains - multiple headers with contains - request cached
    Given request predicates
      ```yaml
      - Header:
          User-Agent:
            contains: Mozilla
          Accept:
            contains: json
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      User-Agent: Mozilla/5.0
      Accept: application/json
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      User-Agent: Mozilla/5.0
      Accept: application/json; charset=utf-8
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Contains - header missing - request not cached
    Given request predicates
      ```yaml
      - Header:
          x-custom-header:
            contains: value
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
