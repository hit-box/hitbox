Feature: Logical Complex Predicate Combinations

  Background:
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```

  # And + Or Combinations

  @integration
  Scenario: And + Or - (Method=GET OR Method=HEAD) AND Path matches
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: GET
            - Method: HEAD
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
  Scenario: And + Or - Method=GET AND (Path=/users OR Path=/books)
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Or:
            - Path: /v1/authors/robert-sheckley/books/victim-prime
            - Path: /v1/authors/robert-sheckley/books
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
  Scenario: And + Or - (Header OR Header) AND Method=GET
    Given request predicates
      ```yaml
      And:
        - Or:
            - Header:
                x-tenant-id: "tenant-a"
            - Header:
                x-tenant-id: "tenant-b"
        - Method: GET
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-tenant-id: tenant-a
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-tenant-id: tenant-a
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: And + Or - Method AND (Header OR Header) AND Path - three level
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Or:
            - Header:
                Content-Type: "application/json"
            - Header:
                Content-Type: "application/xml"
        - Path: /v1/authors/robert-sheckley/books/victim-prime
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
      Content-Type: application/json
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  # Not + Or Combinations

  @integration
  Scenario: Not + Or - Not(Method=POST OR Method=DELETE) caches GET
    Given request predicates
      ```yaml
      Not:
        Or:
          - Method: POST
          - Method: DELETE
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
  Scenario: Not + Or - Not(Method=POST OR Method=DELETE) doesn't cache POST
    Given request predicates
      ```yaml
      Not:
        Or:
          - Method: POST
          - Method: DELETE
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Not + Or - Not(Path OR Path) excludes multiple paths
    Given request predicates
      ```yaml
      Not:
        Or:
          - Path: /v1/authors/robert-sheckley/books/invalid-book-id
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
  Scenario: Not + Or - Method=GET AND Not(Path OR Query)
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Not:
            Or:
              - Path: /v1/authors/robert-sheckley/books/invalid-book-id
              - Query:
                  page: "999"
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

  # Not + And Combinations

  @integration
  Scenario: Not + And - Not(Method=POST AND Path) - DeMorgan's law
    Given request predicates
      ```yaml
      Not:
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
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Not + And - Not(Method=POST AND Path) doesn't match POST with matching path
    Given request predicates
      ```yaml
      Not:
        And:
          - Method: POST
          - Path: /v1/authors/robert-sheckley/books/new-book-test
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Not + And - Not(Header exists AND Method=DELETE)
    Given request predicates
      ```yaml
      Not:
        And:
          - Header: "x-tenant-id"
          - Method: DELETE
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-tenant-id: tenant-a
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-tenant-id: tenant-a
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Not + And - Method=GET AND Not(Header AND Query)
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Not:
            And:
              - Header:
                  x-no-cache: "true"
              - Query:
                  debug: "1"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      x-no-cache: true
      [Query]
      debug: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Not + And - Method=GET AND Not(Header AND Query) - only one condition met
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Not:
            And:
              - Header:
                  x-no-cache: "true"
              - Query:
                  debug: "1"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      debug: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      debug: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  # Three-Level Nesting

  @integration
  Scenario: Three-level - And(Or(Method, Method), Not(Query), Header)
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: GET
            - Method: HEAD
        - Not:
            Query:
              nocache: "true"
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
  Scenario: Three-level - And(Or(Method, Method), Not(Query), Header) - Or fails before Not evaluated
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: POST
            - Method: PUT
        - Not:
            Query:
              skip-cache: "yes"
        - Header:
            x-test-header: "test-value"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/immortality-inc
      x-test-header: test-value
      [Query]
      skip-cache: yes
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Three-level - Or(And(Method, Path), And(Method, Header))
    Given request predicates
      ```yaml
      Or:
        - And:
            - Method: GET
            - Path: /v1/authors/robert-sheckley/books/victim-prime
        - And:
            - Method: POST
            - Header:
                x-admin: "true"
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
  Scenario: Three-level - Or(And(Method, Path), And(Method, Header)) - second branch matches
    Given request predicates
      ```yaml
      Or:
        - And:
            - Method: GET
            - Path: /v1/authors/robert-sheckley/books/victim-prime
        - And:
            - Method: POST
            - Header:
                x-admin: "true"
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      x-admin: true
      {"title":"Test Book","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      x-admin: true
      {"title":"Different Title","description":"Different Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Three-level - (Method OR Method) AND Not(Path) AND Header
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: GET
            - Method: HEAD
        - Not:
            Path: /v1/authors/robert-sheckley/books/invalid-book-id
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
  Scenario: Three-level - Method AND Body AND Not(Body) AND Or(Header OR Header)
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Body: ".title != null"
        - Not:
            Body: '.title == "IntrospectionQuery"'
        - Or:
            - Header:
                x-tenant-id: "tenant-a"
            - Header:
                x-tenant-id: "tenant-b"
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
      x-tenant-id: tenant-a
      {"title":"Test Book","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      x-tenant-id: tenant-a
      {"title":"Different Title","description":"Different Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Three-level - Method AND Body AND Not(Body) AND Or(Header OR Header) - Not fails
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Body: ".title != null"
        - Not:
            Body: '.title == "IntrospectionQuery"'
        - Or:
            - Header:
                x-tenant-id: "tenant-a"
            - Header:
                x-tenant-id: "tenant-b"
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      x-tenant-id: tenant-a
      {"title":"IntrospectionQuery","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And + Or - (Method OR Method) AND Path - Or fails - not cached
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: POST
            - Method: DELETE
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
  Scenario: And + Or - (Method OR Method) AND Path - Path fails - not cached
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: POST
            - Method: PUT
        - Path: /v1/authors/robert-sheckley/books/victim-prime
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/immortality-inc
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And + Or - Method AND (Path OR Path) - Method fails - not cached
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Or:
            - Path: /v1/authors/robert-sheckley/books/victim-prime
            - Path: /v1/authors/robert-sheckley/books
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: And + Or - Method AND (Path OR Path) - both Or branches fail - not cached
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Or:
            - Path: /v1/authors/robert-sheckley/books/victim-prime
            - Path: /v1/authors/robert-sheckley/books
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/immortality-inc
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Not + Or - Method AND Not(Path OR Header) - both Or branches match so Not fails
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Not:
            Or:
              - Path: /v1/authors/robert-sheckley/books/victim-prime
              - Header:
                  x-skip: "true"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-skip: true
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Not + And - Method AND Not(Header AND Path) - Method fails before Not evaluated
    Given request predicates
      ```yaml
      And:
        - Method: POST
        - Not:
            And:
              - Header:
                  x-no-cache: "true"
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
  Scenario: Three-level - And(Or, Not, Header) - Or fails - not cached
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: POST
            - Method: DELETE
        - Not:
            Query:
              nocache: "true"
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
    And cache has 0 records

  @integration
  Scenario: Three-level - And(Or, Not, Header) - Header fails - not cached
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: POST
            - Method: PUT
        - Not:
            Query:
              skip: "yes"
        - Header:
            x-special-key: "special-value"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      x-special-key: wrong-value
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Three-level - Or(And, And) - both And branches fail - not cached
    Given request predicates
      ```yaml
      Or:
        - And:
            - Method: GET
            - Path: /v1/authors/robert-sheckley/books/victim-prime
        - And:
            - Method: POST
            - Header:
                x-admin: "true"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/immortality-inc
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Three-level - (Method OR Method) AND Not(Path) AND Header - And fails after Not
    Given request predicates
      ```yaml
      And:
        - Or:
            - Method: POST
            - Method: DELETE
        - Not:
            Path: /v1/authors/robert-sheckley/books/invalid-book-id
        - Header:
            x-required-header: "required-value"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
