Feature: Request Header Eq Predicate

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Header Eq - exact match caches request
    Given request predicates
      ```yaml
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
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: secret123
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Eq - different value not cached
    Given request predicates
      ```yaml
      - Header:
          x-api-key: "secret123"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: wrongkey
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header Eq - case-insensitive header name
    Given request predicates
      ```yaml
      - Header:
          X-API-Key: "secret123"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: secret123
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header Eq - multiple header predicates all must match
    Given request predicates
      ```yaml
      - Header:
          x-api-key: "secret123"
          x-tenant-id: "tenant-a"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: secret123
      x-tenant-id: tenant-a
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: secret123
      x-tenant-id: tenant-a
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Eq - header with empty value
    Given request predicates
      ```yaml
      - Header:
          x-empty: ""
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-empty:
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header Eq - multiple header values with EQ operation
    Given request predicates
      ```yaml
      - Header:
          x-custom-header: value2
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-custom-header: value1
      x-custom-header: value2
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header Eq - value case sensitivity
    Given request predicates
      ```yaml
      - Header:
          x-api-key: "Secret123"
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
  Scenario: Header Eq - header with whitespace trimmed
    Given request predicates
      ```yaml
      - Header:
          x-token: "mytoken"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-token:  mytoken
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header Eq - missing header not cached
    Given request predicates
      ```yaml
      - Header:
          x-required-header: "value123"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
