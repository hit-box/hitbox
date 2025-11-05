Feature: Request Header In Predicate

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Header In - value in list cached with same cache entry
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
  Scenario: Header In - value not in list not cached
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
  Scenario: Header In - single value in list
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
  Scenario: Header In - empty list behavior
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
  Scenario: Header In - value case sensitivity
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
  Scenario: Header In - missing header not cached
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

  @integration
  Scenario: Header In - multiple header values with IN operation
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
