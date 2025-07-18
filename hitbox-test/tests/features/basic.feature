Feature: Cache policy feature

  Scenario: first test scenario
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 42
          stale: 43
      ```
    Given handler
      ```yaml
      method: GET
      path: /greet/{name}
      status_code: 200
      headers: {}
      body: Hello, test
      ```
    Given request predicates
      ```yaml
      - Method: GET
      - Query:
          operation: Eq
          x-cache: '42'
          cache: 'true'
      ```
    Given response predicates
      ```yaml
      And:
        - Status: 200
        - Or:
            - Status: 200
            - Status: 203
            - Status: 500
        - Status: 200
      ```
    Given key extractors
      ```yaml
      - Method:
      - Path: /greet/{name}
      ```
    When execute request
      ```hurl
      GET http://localhost/greet/test
      X-Cache-ID: 123
      [Query]
      cache: true
      x-cache: 42
      [Options]
      delay: 3
      {"key": 42}
      ```
    Then response status is 200
    And cache has records
      | name:test,method:GET | Hello, test |
