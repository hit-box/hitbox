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
  # Response-side invalidation. Response tags are extracted at WRITE time and
  # stored on the cache entry; the FSM reads them back on the next cache hit and
  # queries their invalidation timestamps (the lazy post-hit check). These
  # scenarios configure ONLY a response tag, so the second miss can be caused by
  # nothing but that read-back path — positive proof it is wired end-to-end.
  # ============================================================================

  @tag-invalidation @miss @response-side
  Scenario: Cache hit ignored when a response tag is invalidated
    Given tags
      """
      response:
        - Static:
            etag: "v1"
      """
    # First request — miss; the response is cached WITH "etag=v1" stored on the entry.
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # Invalidate the response-derived tag (stored on the entry at write time).
    When sleep 20ms
    And tag "etag=v1" is invalidated
    And sleep 20ms

    # Second request — must miss: the FSM reads the stored response tag back and
    # checks it. No request tag is configured, so this is the only path that can
    # force the miss.
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times

  @tag-invalidation @hit @response-side
  Scenario: Cache hit served when the response tag is not invalidated
    Given tags
      """
      response:
        - Static:
            etag: "v1"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # No invalidation — the stored response tag check passes, second request hits.
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And GetBook should be called 1 time

  @tag-invalidation @hit @response-side
  Scenario: Cache hit served when an unrelated tag is invalidated (response side)
    Given tags
      """
      response:
        - Static:
            etag: "v1"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"

    # Invalidate a different response tag — the stored "etag=v1" is unaffected.
    When sleep 20ms
    And tag "etag=v2" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And GetBook should be called 1 time

  @tag-invalidation @miss @response-side
  Scenario: Request and response tags merged — invalidating the response tag misses
    Given tags
      """
      request:
        - Static:
            user: "42"
      response:
        - Static:
            etag: "v1"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # Invalidate only the response tag. The request tag ("user=42") is checked in
    # parallel and is clean; the response tag is checked lazily after the hit.
    # Both result maps are merged — the response invalidation must still force a miss.
    When sleep 20ms
    And tag "etag=v1" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times

  @tag-invalidation @hit @response-side @skip-policy
  Scenario: TagInvalidation Skip disables the response read-path
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10s
        tag_invalidation: Skip
      ```
    And tags
      """
      response:
        - Static:
            etag: "v1"
      """
    # With Skip, the response tag is neither extracted/stored at write nor checked
    # at read — so invalidating it has no effect and the second request hits.
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    When sleep 20ms
    And tag "etag=v1" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "HIT"
    And GetBook should be called 1 time

  # ============================================================================
  # Invalidation during an in-flight upstream call. The upstream may have read
  # the data before it changed, so the entry that call produces must not
  # outlive the invalidation. Entries are stamped with the time the upstream
  # call started, not when its response arrived.
  # ============================================================================

  @tag-invalidation @miss @race
  Scenario: Invalidation during an in-flight upstream call expires the resulting entry
    Given tags
      """
      request:
        - Static:
            user: "42"
      """
    And upstream delay for GetBook is 200ms
    # The tag is invalidated 50ms into the 200ms upstream call.
    When request is executed while tag "user=42" is invalidated after 50ms
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    # The cached entry may hold pre-invalidation data, so it must not be served.
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times

  # ============================================================================
  # Dynamic response tags: a response Header extractor derives the tag from the
  # actual response header (here `etag: book-v1`), reusing the same header
  # machinery as the request side. Invalidating the derived tag misses.
  # ============================================================================

  @tag-invalidation @miss @response-side @dynamic
  Scenario: Response Header tag extractor — invalidating the header-derived tag misses
    Given tags
      """
      response:
        - Header: "etag"
      """
    # The handler returns "etag: book-v1"; the extractor stores tag "etag=book-v1".
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime?test_headers=true
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 1 time

    When sleep 20ms
    And tag "etag=book-v1" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime?test_headers=true
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And GetBook should be called 2 times

  @tag-invalidation @hit @response-side @dynamic
  Scenario: Response Header tag extractor — invalidating an unrelated version is a hit
    Given tags
      """
      response:
        - Header: "etag"
      """
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime?test_headers=true
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"

    # Invalidate a different entity version — the stored "etag=book-v1" is unaffected.
    When sleep 20ms
    And tag "etag=book-v2" is invalidated
    And sleep 20ms

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime?test_headers=true
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
