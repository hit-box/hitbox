Feature: Cache policy feature

  Scenario: first test scenario
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 42
          stale: 43
      ```
    Given request predicates
      | method | GET   |
      | query  | cache |
    Given key extractor "method"
    Given key extractor "path=/greet/{name}"
    When execute request
      ```hurl
      GET /greet/test
      X-Cache-ID: 123
      [Options]
      delay: 3
      ```
    Then response status is 200
    And cache has records
      | test | value |
    # And cache has record
    #   | X-Auth-Token | secret123        |
    #   | Content-Type | application/json |
    # Given request predicate method=GET
    # Given request predicate query=cache
    # When I send a GET request to "/greet/max?type=book"
    # And I set headers:
    #   | X-Auth-Token | secret123        |
    #   | Content-Type | application/json |
    # And the request body is:
    #   """
    #   { "title": "Rust Book", "author": "Ferris" }
    #   """
    # And execute request
    # Then the response status should be 200
