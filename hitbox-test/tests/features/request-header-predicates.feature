Feature: Request Header Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```

  @integration
  Scenario: Header Eq operation - exact match caches request
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
  Scenario: Header Eq operation - different value not cached
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
  Scenario: Header Eq operation - case-insensitive header name
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
  Scenario: Header Exist operation - presence check caches request
    Given request predicates
      ```yaml
      - Header: "Authorization"
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Authorization: Bearer any-token-here
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Authorization: Bearer different-token
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Exist operation - missing header not cached
    Given request predicates
      ```yaml
      - Header: "Authorization"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header In operation - value in list cached with different keys
    Given request predicates
      ```yaml
      - Header:
          Content-Type:
            - application/json
            - application/xml
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Content-Type: application/json
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Content-Type: application/xml
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And cache has 1 records

  @integration
  Scenario: Header In operation - value not in list not cached
    Given request predicates
      ```yaml
      - Header:
          Content-Type:
            - application/json
            - application/xml
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Content-Type: text/html
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Multiple header predicates - all must match
    Given request predicates
      ```yaml
      - Header:
          x-api-key: "secret123"
      - Header:
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
      x-tenant-id: tenant-b
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header with empty value
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
  Scenario: Additional header doesn't affect cache decision
    Given request predicates
      ```yaml
      - Header: "Authorization"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Authorization: Bearer token123
      User-Agent: Mozilla/5.0
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Multiple header values with EQ operation
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
  Scenario: Multiple header values with IN operation
    Given request predicates
      ```yaml
      - Header:
          x-custom-header:
              - value3
              - value2
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
  Scenario: Header Eq operation - value case sensitivity
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
  Scenario: Header Eq operation - header with leading and trailing whitespace trimmed
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
  Scenario: Header Eq operation - missing header not cached
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

  @integration
  Scenario: Header Exist operation - accepts any value
    Given request predicates
      ```yaml
      - Header: "X-Trace-Id"
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      X-Trace-Id: trace-abc-123
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      X-Trace-Id: completely-different-value
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Exist operation - case-insensitive header name
    Given request predicates
      ```yaml
      - Header: "Authorization"
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      authorization: Bearer token123
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header In operation - single value in list
    Given request predicates
      ```yaml
      - Header:
          Accept:
            - application/json
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
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header In operation - empty list behavior
    Given request predicates
      ```yaml
      - Header:
          X-Feature-Flag: []
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      X-Feature-Flag: enabled
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header In operation - value case sensitivity
    Given request predicates
      ```yaml
      - Header:
          Content-Type:
            - application/json
            - application/xml
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      Content-Type: Application/JSON
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Header In operation - missing header not cached
    Given request predicates
      ```yaml
      - Header:
          Accept-Language:
            - en-US
            - fr-FR
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
