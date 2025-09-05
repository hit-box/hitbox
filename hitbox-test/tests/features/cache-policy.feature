Feature: Cache policy

  @integration
  Scenario: cache is OFF
    Given hitbox with policy
      ```yaml
      !Disabled
      ```
    When execute request
      ```hurl
			GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
		And response body jq '.title=="Victim Prime"'
		And response headers have no "X-Cache" header
    And cache has 0 records

  @integration
  Scenario: cache is ON
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
		And response headers have no "X-Cache" header
    And cache has 1 records
		And cache key "method=GET:author_id=robert-sheckley:book_id=victim-prime" exists
    When execute request
      ```hurl
			GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
		And response body jq '.title=="Victim Prime"'
		And response headers contain "X-Cache" header
