Feature: Logical And Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: And predicate - both predicates match - request cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
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
  Scenario: And predicate - first predicate matches, second doesn't - request not cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Path: /v1/different-path
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - first predicate doesn't match - short-circuit - request not cached
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Path: /v1/authors/robert-sheckley/books/victim-prime
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - both predicates don't match - request not cached
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Path: /v1/different-path
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - three predicates all match - request cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Header:
            x-api-key: "secret123"
        - Path: /v1/authors/robert-sheckley/books/victim-prime
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
  Scenario: And predicate - Method AND Header AND Query all match
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Header:
            x-tenant-id: "tenant-a"
        - Query:
            page: { eq: "1" }
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      x-tenant-id: tenant-a
      [Query]
      page: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      x-tenant-id: tenant-a
      [Query]
      page: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: And predicate - Method AND Path AND Body all match
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Path: /v1/authors/robert-sheckley/books/new-book-test
        - Body: ".title != null"
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Different Title","description":"Different Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: And predicate - three predicates - first fails - not cached
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Header:
            x-api-key: "secret123"
        - Path: /v1/authors/robert-sheckley/books/victim-prime
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
  Scenario: And predicate - three predicates - second fails - not cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Header:
            x-api-key: "secret123"
        - Path: /v1/authors/robert-sheckley/books/victim-prime
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-api-key: wrong-key
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - three predicates - third fails - not cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Header:
            x-api-key: "secret123"
        - Path: /v1/authors/robert-sheckley/books/victim-prime
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/immortality-inc
      x-api-key: secret123
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - Method AND Header AND Query - Query fails - not cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Header:
            x-tenant-id: "tenant-a"
        - Query:
            page: { eq: "1" }
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      x-tenant-id: tenant-a
      [Query]
      page: 2
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And predicate - Method AND Path AND Body - Body fails - not cached
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Path: /v1/authors/robert-sheckley/books/new-book-negative-test
        - Body: '.description == "Specific Description"'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-negative-test
      Content-Type: application/json
      {"title":"Test Book","description":"Different Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
