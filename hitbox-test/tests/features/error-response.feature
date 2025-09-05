Feature: Error Response Caching Behavior

  @integration
  Scenario: HTTP 404 responses should not be cached
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 120
      ```
    When execute request
      ```hurl
			GET http://localhost/v1/authors/robert-sheckley/books/unknown
      ```
    Then response status is 404
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records

  @integration
  Scenario: HTTP 500 responses should not be cached
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 120
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/invalid-book-id
      ```
    Then response status is 500
    And response header "X-Cache-Status" is "MISS"
    And cache has 0 records
