Feature: HTTP Response Caching Policy Configuration

  @integration
  Scenario: Disabled cache policy should not store responses
    Given hitbox with policy
      ```yaml
      !Disabled
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200 And response body jq '.title=="Victim Prime"'
    And response headers have no "X-Cache-Status" header
    And cache has 0 records

  @integration
  Scenario: Enabled cache policy should store and retrieve responses
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 120
          stale: 60
      ```
    And key extractors
      ```yaml
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      - Method:
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response body jq '.title=="Victim Prime"'
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    And cache key "method=GET:author_id=robert-sheckley:book_id=victim-prime" exists
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response body jq '.title=="Victim Prime"'
    And response header "X-Cache-Status" is "HIT"

  @integration @serial
  Scenario: Enabled cache policy should use ttl
    Given mock time is enabled
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 2
      ```
    And key extractors
      ```yaml
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      - Method:
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response body jq '.title=="Victim Prime"'
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records
    And cache key "method=GET:author_id=robert-sheckley:book_id=victim-prime" exists
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response body jq '.title=="Victim Prime"'
    And response header "X-Cache-Status" is "HIT"
    When sleep 3
    And execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
