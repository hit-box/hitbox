Feature: Tag-Based Cache Invalidation

  Verifies that tag extractors configured via the `tags:` section drive
  cache invalidation correctly. Each scenario primes the cache with an
  initial request, optionally invalidates a tag, then issues a second
  request and asserts the resulting cache status.

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10s
      ```

  @tag-invalidation @miss
  Scenario: Cache hit ignored when a request tag is invalidated
    Given tags
      """
      request:
        - Static:
            user: "42"
      """
    # First request — cache miss, populates entry
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # Invalidate the tag — write timestamp is now newer than entry.created
    When sleep 20ms
    And tag "user=42" is invalidated
    And sleep 20ms

    # Second request — must miss because of tag invalidation
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times

  @tag-invalidation @hit
  Scenario: Cache hit served when no tag is invalidated
    Given tags
      """
      request:
        - Static:
            user: "42"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # No invalidation — second request should hit
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And GetBook should be called 1 time

  @tag-invalidation @hit
  Scenario: Cache hit served when an unrelated tag is invalidated
    Given tags
      """
      request:
        - Static:
            user: "42"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"

    # Invalidate a different tag — should not affect this entry
    When sleep 20ms
    And tag "user=99" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And GetBook should be called 1 time

  @tag-invalidation @miss
  Scenario: Cache hit ignored when any of multiple request tags is invalidated
    Given tags
      """
      request:
        - Static:
            user: "42"
            region: "eu"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # Invalidate one of the two tags — entry must be treated as expired
    When sleep 20ms
    And tag "region=eu" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times

  @tag-invalidation @hit
  Scenario: No tag extractor configured — invalidation has no effect
    # Default neutral tag extractor: request emits no tags, no parallel
    # invalidation prefetch, behavior unchanged from non-tag flows.
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # Even invalidating an arbitrary tag does nothing because the request
    # extractor produces no tags to compare against.
    When sleep 20ms
    And tag "user=42" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And GetBook should be called 1 time

  # ============================================================================
  # Compatibility layer: any Extractor variant can serve as a tag extractor
  # via TagAdapter. Here Path emits author_id / book_id KeyParts; those become
  # CacheTags of the form "author_id=robert-sheckley" / "book_id=victim-prime".
  # ============================================================================

  @tag-invalidation @miss @compat-layer
  Scenario: Path extractor reused as tag extractor — invalidating path-derived tag misses
    Given tags
      """
      request:
        - Path: "/v1/authors/{author_id}/books/{book_id}"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # Invalidate one of the path-derived tags
    When sleep 20ms
    And tag "author_id=robert-sheckley" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times

  @tag-invalidation @hit @compat-layer
  Scenario: Path extractor as tag extractor — invalidating an unrelated path tag is a hit
    Given tags
      """
      request:
        - Path: "/v1/authors/{author_id}/books/{book_id}"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"

    # Invalidate a tag for a *different* author — should not affect this entry
    When sleep 20ms
    And tag "author_id=someone-else" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And GetBook should be called 1 time

  @tag-invalidation @miss @compat-layer
  Scenario: Static + Path mixed — chained tag extractors all contribute to invalidation
    Given tags
      """
      request:
        - Static:
            tier: "premium"
        - Path: "/v1/authors/{author_id}/books/{book_id}"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # Invalidate the static tag — chain produced both static and path-derived
    # tags; invalidating any one of them must miss.
    When sleep 20ms
    And tag "tier=premium" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times
