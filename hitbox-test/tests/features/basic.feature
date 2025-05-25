Feature: Cache policy feature

  Scenario: first test scenario
    Given hitbox with policy 42
      # ```yaml
      # enabled: true
      # ```
    Given request predicate method=GET
    Given request predicate query=cache
    When I send a GET request to "/greet/max?type=book"
    And I set headers:
      | X-Auth-Token | secret123        |
      | Content-Type | application/json |
    And the request body is:
      """
      { "title": "Rust Book", "author": "Ferris" }
      """
    And execute request
    Then the response status should be 200
