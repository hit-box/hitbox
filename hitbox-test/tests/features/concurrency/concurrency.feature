@serial @concurrency
Feature: Concurrency Control (Dogpile Prevention)

  Background:
    Given request predicates
      ```yaml
      - Method: GET
      - Path: /v1/authors/{author_id}/books/{book_id}
      ```
    And response predicates
      ```yaml
      - Status: 200
      ```
    And key extractors
      ```yaml
      - Method:
      - Path: "/v1/authors/{author_id}/books/{book_id}"
      ```

  Scenario: Concurrent requests with concurrency limit of 1 - only one upstream call
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 300s
        concurrency: 1
      ```
    And upstream delay for GetBook is 35ms
    When 3 concurrent requests are made with delay 10ms
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then all responses should have status 200
    And GetBook should be called 1 time
    And backend read was called 3 times with all miss
    And backend write was called 1 times
    And response headers are
      | X-Cache-Status | MISS |
      | X-Cache-Status | HIT  |
      | X-Cache-Status | HIT  |

  Scenario: Concurrent requests with concurrency limit of 3 - all requests go to upstream
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 300s
        concurrency: 3
      ```
    And upstream delay for GetBook is 35ms
    When 3 concurrent requests are made with delay 10ms
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then all responses should have status 200
    And GetBook should be called 3 times
    And backend read was called 3 times with all miss
    And backend write was called 3 times
    And response headers are
      | X-Cache-Status | MISS |
      | X-Cache-Status | MISS |
      | X-Cache-Status | MISS |

  Scenario: Third request hits cache after first completes
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 300s
        concurrency: 1
      ```
    And upstream delay for GetBook is 15ms
    When 3 concurrent requests are made with delay 10ms
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then all responses should have status 200
    And GetBook should be called 1 time
    And backend write was called 1 times
    And response headers are
      | X-Cache-Status | MISS |
      | X-Cache-Status | HIT  |
      | X-Cache-Status | HIT  |

  Scenario: Non-cacheable response (status 201) - no coalescing, all go to upstream
    Given response predicates
      ```yaml
      - Status: 201
      ```
    And hitbox with policy
      ```yaml
      Enabled:
        ttl: 300s
        concurrency: 1
      ```
    And upstream delay for GetBook is 35ms
    When 3 concurrent requests are made with delay 10ms
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then all responses should have status 200
    And GetBook should be called 3 times
    And backend write was called 0 times
