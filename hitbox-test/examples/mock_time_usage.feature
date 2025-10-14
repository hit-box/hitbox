# Example feature file demonstrating MockTime usage

Feature: Cache TTL with Mock Time
  Test cache expiration using mock time instead of real sleep

  Background:
    Given mock time is enabled
    And hitbox with policy
    """
    Enabled:
      ttl: 10
    """

  Scenario: Cache expires after TTL using mock time
    # First request - cache miss, stores in cache
    When execute request
    """
    GET http://localhost/books
    """
    Then response status code is 200
    And response headers contain a header "X-Cache-Status" with value "MISS"

    # Second request immediately - cache hit
    When execute request
    """
    GET http://localhost/books
    """
    Then response status code is 200
    And response headers contain a header "X-Cache-Status" with value "HIT"

    # Advance time by 5 seconds (still within TTL)
    When sleep 5
    When execute request
    """
    GET http://localhost/books
    """
    Then response status code is 200
    And response headers contain a header "X-Cache-Status" with value "HIT"

    # Advance time by 6 more seconds (total 11 seconds, past TTL)
    When sleep 6
    When execute request
    """
    GET http://localhost/books
    """
    Then response status code is 200
    And response headers contain a header "X-Cache-Status" with value "MISS"

  Scenario: Test with real sleep (mock time disabled)
    Given mock time is disabled
    # This will use actual tokio::time::sleep
    When sleep 2
    When execute request
    """
    GET http://localhost/books
    """
    Then response status code is 200
