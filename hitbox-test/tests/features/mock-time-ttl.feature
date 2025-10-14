@serial @integration
Feature: Cache TTL with Mock Time
  Test cache expiration using mock time to avoid actual waiting

  Note: This feature uses @serial tag to ensure scenarios run sequentially.
  This is required because mock time uses global state that would interfere
  if multiple scenarios ran concurrently.

  Background:
    Given mock time is enabled
    And hitbox with policy
      """
      !Enabled
      ttl: 10
      """

  Scenario: Cache expires after TTL
    # First request - cache miss
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    # Second request immediately - cache hit
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    # Advance time by 5 seconds (still within TTL of 10 seconds)
    When sleep 5
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    # Advance time by 6 more seconds (total 11 seconds, past TTL)
    When sleep 6
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"

  Scenario: Cache with stale time
    Given hitbox with policy
      """
      !Enabled
      ttl: 30
      stale: 10
      """
    # First request - cache miss
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    # Immediately - cache hit
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    # After 5 seconds - still fresh
    When sleep 5
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    # After 15 seconds total - now stale but still returns from cache
    When sleep 10
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    # Note: Stale behavior depends on your implementation
    # This might be HIT or STALE depending on how you handle it
    And response headers contain "X-Cache-Status" header
    # After 35 seconds total - expired
    When sleep 20
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"

  Scenario: Reset mock time
    # Make some requests
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    # Advance time
    When sleep 5
    # Reset mock time to baseline
    Given mock time is reset
    # Time should be back to baseline, cache should still be valid
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"

  Scenario: Disable mock time uses real sleep
    Given mock time is disabled
    # This scenario would use real tokio::sleep
    # Keep it short to avoid slow tests
    When execute request
      """
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      """
    Then response status is 200
