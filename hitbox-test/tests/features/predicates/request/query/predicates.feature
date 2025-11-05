Feature: Request Query Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Single query parameter match
    Given request predicates
      ```yaml
      - Query:
          cache: { eq: "true" }
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
			[Query]
			cache: true
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
			[Query]
			cache: true
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Multiple query parameters match
    Given request predicates
      ```yaml
      - Query:
          page: "1"
          per_page: "10"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      per_page: 10
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      per_page: 10
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Query parameter value mismatch not cached
    Given request predicates
      ```yaml
      - Query:
          page: "1"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 2
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Empty query parameter value
    Given request predicates
      ```yaml
      - Query:
          empty: ""
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      empty:
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Value in list cached
    Given request predicates
      ```yaml
      - Query:
          page: !in
            - "1"
            - "2"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 2
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Value not in list not cached
    Given request predicates
      ```yaml
      - Query:
          page:
            - "1"
            - "2"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 3
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Empty list behavior
    Given request predicates
      ```yaml
      - Query:
          page: []
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Query parameter exists cached
    Given request predicates
      ```yaml
      - Query:
          page: { exists: }
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 2
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Query parameter missing not cached
    Given request predicates
      ```yaml
      - Query:
          page: { exists: }
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
