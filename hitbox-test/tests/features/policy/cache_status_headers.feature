@serial
Feature: Cache-Status and Age Headers (RFC 9211 / RFC 9111)

  Verifies that the Cache-Status structured header (RFC 9211) and Age header
  (RFC 9111 §5.1) are correctly generated for all cache scenarios.

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10s
      ```

  # ===========================================================================
  # Group 1: Forward scenarios (fwd=miss)
  # ===========================================================================

  @cache-status @forward @miss
  Scenario: Cache miss - first request produces fwd=miss with stored
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=200; stored"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

  @cache-status @forward @not-stored
  Scenario: Non-cacheable response (404) - fwd=miss without stored
    Given response predicates
      ```yaml
      - Status: 200
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/nonexistent-book
      ```
    Then response status is 404
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=404"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

  @cache-status @forward @not-stored @500
  Scenario: Upstream error (500) - fwd=miss without stored
    Given response predicates
      ```yaml
      - Status: 200
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/invalid-book-id
      ```
    Then response status is 500
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=500"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

  @cache-status @forward @not-stored @empty-body
  Scenario: Empty list response rejected by body predicate - fwd=miss without stored
    Given response predicates
      ```yaml
      - Body:
          jq: 'length > 0'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books?page=999
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=200"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

  @cache-status @forward @bypass
  Scenario: Request predicate bypass - fwd=bypass
    Given request predicates
      ```yaml
      - Header:
          name: X-Custom
          eq: "cache-me"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" starts with "hitbox; fwd=bypass"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

  @cache-status @forward @expired
  Scenario: Expired cache entry - fwd=stale (RFC 9211 uses "stale" for expired forwards)
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 200ms
      ```
    # First request - miss, stored
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=200; stored"

    # Wait past TTL - entry expired
    When sleep 250ms
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=stale; fwd-status=200; stored"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

  @cache-status @forward @disabled
  Scenario: Cache disabled - fwd=bypass without stored
    Given hitbox with policy
      ```yaml
      Disabled
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" starts with "hitbox; fwd=bypass"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

  @cache-status @forward @revalidate
  Scenario: Stale entry with Revalidate policy - synchronous forward
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 300ms
        stale: 100ms
        policy:
          stale: Revalidate
      ```
    # First request - miss
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=200; stored"

    # Wait past stale mark - Revalidate policy forwards synchronously
    When sleep 150ms
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=stale; fwd-status=200; stored"
    And response header "X-Cache-Status" is "MISS"

  # ===========================================================================
  # Group 2: Hit scenarios
  # ===========================================================================

  @cache-status @hit
  Scenario: Cache hit - second request produces hit with ttl
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" starts with "hitbox; hit; ttl="
    And response header "X-Cache-Status" is "HIT"
    And response headers contain "Age" header

  @cache-status @hit @no-ttl
  Scenario: Cache hit without expire - hit without ttl parameter
    Given hitbox with policy
      ```yaml
      Enabled: {}
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; hit"
    And response header "X-Cache-Status" is "HIT"
    And response headers contain "Age" header

  @cache-status @hit @lifecycle
  Scenario: Full miss-to-hit lifecycle in a single scenario
    # First request - miss, stored
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=200; stored"
    And response header "X-Cache-Status" is "MISS"
    And response headers have no "Age" header
    And cache has 1 records

    # Second request - hit
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" starts with "hitbox; hit; ttl="
    And response header "X-Cache-Status" is "HIT"
    And response headers contain "Age" header

  # ===========================================================================
  # Group 3: Age header
  # ===========================================================================

  @cache-status @age
  Scenario: Age is 0 on immediate cache hit
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response headers have no "Age" header

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Age" is "0"

  @cache-status @age @increases
  Scenario: Age increases over time
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Age" is "0"

    When sleep 1000ms
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Age" is "1"

  # ===========================================================================
  # Group 4: Stale scenarios
  # ===========================================================================

  @cache-status @stale @swr
  Scenario: Stale-while-revalidate (OffloadRevalidate) - hit with stale status
    Given offload revalidation is enabled
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 300ms
        stale: 100ms
        policy:
          stale: OffloadRevalidate
      ```
    # Miss
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=200; stored"

    # Past stale mark (100ms) but within TTL (300ms) - serves stale
    When sleep 150ms
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" starts with "hitbox; hit; ttl="
    And response header "X-Cache-Status" is "STALE"
    And response headers contain "Age" header

  @cache-status @stale @return
  Scenario: Stale-while-revalidate (Return policy) - serves stale without background refresh
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 300ms
        stale: 100ms
        policy:
          stale: Return
      ```
    # Miss
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" is "hitbox; fwd=miss; fwd-status=200; stored"

    # Past stale mark - serves stale
    When sleep 150ms
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response header "Cache-Status" starts with "hitbox; hit; ttl="
    And response header "X-Cache-Status" is "STALE"
    And response headers contain "Age" header

  # ===========================================================================
  # Group 5: Collapsed (dog-pile prevention)
  # ===========================================================================

  @cache-status @collapsed
  Scenario: Collapsed requests - first goes upstream, others wait
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
    And GetBook should be called 1 time
    And response headers start with
      | Cache-Status | hitbox; fwd=miss          |
      | Cache-Status | hitbox; hit; collapsed |
      | Cache-Status | hitbox; hit; collapsed |

  # ===========================================================================
  # Group 6: Legacy header coexistence
  # ===========================================================================

  @cache-status @legacy
  Scenario: Both Cache-Status and X-Cache-Status headers are present on miss
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response headers contain "Cache-Status" header
    And response headers contain "X-Cache-Status" header
    And response header "X-Cache-Status" is "MISS"

  @cache-status @legacy
  Scenario: Both Cache-Status and X-Cache-Status headers are present on hit
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response headers contain "Cache-Status" header
    And response headers contain "X-Cache-Status" header
    And response header "X-Cache-Status" is "HIT"
