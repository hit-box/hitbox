Feature: Cache policy feature

  Scenario: first test scenario
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 42
          stale: 43
      ```
    Given request predicates
      ```yaml
      - Method: GET
      - Query:
          operation: Eq
          x-cache: '42'
          cache: 'true'
      ```
    # Given request predicates
    #   ```yaml
    #   And:
    #   - Method: GET
    #   - Query:
    #       operation: Eq
    #       cache: 'true'
    #   ```
    Given key extractor "method"
    Given key extractor "path=/greet/{name}"
    When execute request
      ```hurl
      GET http://localhost/greet/test
      X-Cache-ID: 123
      [Options]
      delay: 3
      {"key": 42}
      ```
    Then response status is 200
    And cache has records
      | test | value |
