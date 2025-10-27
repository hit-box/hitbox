Feature: Request Body Predicate Functionality

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Body Eq operation - simple field exact match cached
    Given request predicates
      ```yaml
      - Body: '.field == "test-value"'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-1
      Content-Type: application/json
      {"title":"Test Book","description":"Test description","field":"test-value"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-1
      Content-Type: application/json
      {"title":"Test Book","description":"Test description","field":"test-value"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body Eq operation - field value mismatch not cached
    Given request predicates
      ```yaml
      - Body: '.field == "expected-value"'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","field":"wrong-value"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Body Eq operation - nested object field match cached
    Given request predicates
      ```yaml
      - Body: '.inner.field_one == "value_one"'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","inner":{"field_one":"value_one","field_two":"value_two"}}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","inner":{"field_one":"value_one","field_two":"value_two"}}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body Eq operation - array element match cached
    Given request predicates
      ```yaml
      - Body: '.items[1].key == "my-key-01"'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","items":[{"key":"my-key-00","value":"my-value-00"},{"key":"my-key-01","value":"my-value-01"}]}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","items":[{"key":"my-key-00","value":"my-value-00"},{"key":"my-key-01","value":"my-value-01"}]}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body Eq operation - missing field not cached
    Given request predicates
      ```yaml
      - Body: '.missing_field == "value"'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","other_field":"value"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Body Eq operation - number value match
    Given request predicates
      ```yaml
      - Body: '.count == 42'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","count":42}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Body Eq operation - boolean value match
    Given request predicates
      ```yaml
      - Body: '.active == true'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","active":true}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Body Eq operation - null value match
    Given request predicates
      ```yaml
      - Body: '.value == null'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","value":null}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Body Eq operation - multiple body predicates all must match
    Given request predicates
      ```yaml
      - Body: '.user == "alice"'
      - Body: '.role == "admin"'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","user":"alice","role":"admin"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-multi-pred
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","user":"alice","role":"user"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Body Eq operation - complex jq expression match
    Given request predicates
      ```yaml
      - Body: '.items | length > 2'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","items":[1,2,3,4]}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-complex-jq
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","items":[1,2,3,4]}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And cache has 1 records

  @integration
  Scenario: Body Eq operation - missing field equals null
    Given request predicates
      ```yaml
      - Body: '.extra == null'
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
      {"title":"Test Book","description":"Test Description"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And cache has 1 records

  @integration
  Scenario: Body Exist operation - field exists cached any value
    Given request predicates
      ```yaml
      - Body: '.token != null'
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
      {"title":"Test Book","description":"Test Description","token":"abc123"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","token":"xyz789"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body Exist operation - missing field not cached
    Given request predicates
      ```yaml
      - Body: '.required_field != null'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","other_field":"value"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Body Exist operation - nested field existence
    Given request predicates
      ```yaml
      - Body: '.user.id != null'
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
      {"title":"Test Book","description":"Test Description","user":{"id":123,"name":"alice"}}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","user":{"id":456,"name":"bob"}}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body Exist operation - array element existence
    Given request predicates
      ```yaml
      - Body: '.tags[0] != null'
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
      {"title":"Test Book","description":"Test Description","tags":["fiction","scifi"]}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","tags":["adventure","comedy"]}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body In operation - value in list cached with same key
    Given request predicates
      ```yaml
      - Body: '(.status == "active") or (.status == "pending")'
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
      {"title":"Test Book","description":"Test Description","status":"active"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","status":"pending"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body In operation - value not in list not cached
    Given request predicates
      ```yaml
      - Body: '(.role == "admin") or (.role == "moderator")'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","role":"user"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: Body In operation - multiple data types in list
    Given request predicates
      ```yaml
      - Body: '(.priority == 1) or (.priority == "high") or (.priority == true)'
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
      {"title":"Test Book","description":"Test Description","priority":1}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","priority":"high"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Body In operation - empty list behavior
    Given request predicates
      ```yaml
      - Body: 'false'
      ```
    When execute request
      ```hurl
      POST http://localhost/v1/authors/robert-sheckley/books/new-book-test
      Content-Type: application/json
      {"title":"Test Book","description":"Test Description","type":"book"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
