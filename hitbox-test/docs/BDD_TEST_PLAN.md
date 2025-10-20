# Hitbox Project - BDD Test Implementation Plan

**Generated**: 2025-10-14
**Version**: 1.0

---

## TABLE OF CONTENTS

1. [Project Overview](#project-overview)
2. [Current State Analysis](#current-state-analysis)
3. [Detailed Implementation Plan](#detailed-implementation-plan)
4. [Prioritization Matrix](#prioritization-matrix)
5. [Implementation Statistics](#implementation-statistics)
6. [Recommended Timeline](#recommended-timeline)

---

## PROJECT OVERVIEW

**Hitbox** is an asynchronous HTTP caching framework built on Tower middleware for Rust. It provides sophisticated caching capabilities with:

### Core Architecture
- **Multi-layered design**: Core traits → HTTP layer → Tower integration → Configuration
- **Flexible predicates**: Determine what requests/responses to cache based on method, path, headers, query params, and body content
- **Smart extractors**: Generate cache keys from request components
- **Policy-driven**: TTL and stale cache mechanics with time-mocking support for testing
- **Backend-agnostic**: Currently supports Moka (in-memory) and Redis

### Key Components
1. **hitbox-core**: Foundation traits (Predicate, Extractor, TimeProvider)
2. **hitbox-http**: HTTP-specific predicates and extractors
3. **hitbox-tower**: Tower middleware integration
4. **hitbox-configuration**: YAML-based configuration system
5. **hitbox-test**: BDD test infrastructure using Cucumber

### Test Infrastructure

#### Available BDD Steps

**Given Steps** (from `src/steps/given.rs`):
- `hitbox with policy` - Configure cache policy from YAML
- `request predicates` - Set request predicates from YAML
- `response predicates` - Set response predicates from YAML
- `key extractors` - Set extractors from YAML
- `mock time is enabled` - Enable mock time for testing
- `mock time is disabled` - Disable mock time (use real time)
- `mock time is reset` - Reset mock time to baseline

**When Steps** (from `src/steps/when.rs`):
- `execute request` - Execute HURL format request
- `sleep {int}` - Advance mock time or real sleep

**Then Steps** (from `src/steps/then.rs`):
- `response status is {int}` - Check HTTP status code
- `response body jq {string}` - Validate body with JQ expression
- `response header {string} is {string}` - Check header value
- `response headers contain {string} header` - Check header exists
- `response headers have no {string} header` - Check header absent
- `cache has {int} records` - Verify cache size
- `cache key {string} exists` - Check specific cache key exists

#### HURL Request Format

Requests use HURL format:
```hurl
GET http://localhost/api/users/123
[Query]
page: 1
size: 10
[Headers]
x-api-key: secret
```

#### YAML Configuration Format

**Policy Configuration**:
```yaml
!Enabled
ttl: 120        # Seconds
stale: 60       # Seconds
```

or:

```yaml
!Disabled
```

**Request Predicates**:
```yaml
- Method: GET
- Path: /api/users/{id}
- Header:
    x-api-key: "secret"
- Query:
    operation: Eq
    page: "1"
```

**Response Predicates**:
```yaml
- Status: 200
- Body:
    parser: Jq
    jq: ".error"
    operation: Exist
```

**Extractors**:
```yaml
- Method:
- Path: "/api/users/{id}"
- Query: "page"
- Query: "size"
```

**Conditional Predicates**:
```yaml
Or:
  - Method: GET
  - Method: HEAD
```

---

## CURRENT STATE ANALYSIS

### Existing BDD Test Coverage

**Current feature files** (in `tests/features/`):

1. **cache-policy.feature** (83 lines)
   - ✅ Disabled cache policy
   - ✅ Enabled cache with TTL and stale
   - ✅ TTL expiration with mock time

2. **method-predicate.feature** (46 lines)
   - ✅ GET method matching
   - ✅ HEAD method matching

3. **query-parameter-caching.feature** (39 lines)
   - ✅ Different query params generate distinct keys
   - ✅ Tests extractors: Path, Method, Query

4. **error-response.feature** (32 lines)
   - ✅ 404 responses not cached
   - ✅ 500 responses not cached

5. **mock-time-ttl.feature** (125 lines)
   - ✅ Cache expiration with mock time
   - ✅ Stale cache behavior (basic)
   - ✅ Mock time reset
   - ✅ Mock time disable

**Total**: ~20 scenarios across 6 feature files

### Unit Test Coverage (Strong)

**Request Predicates** (all have comprehensive unit tests in `hitbox-http/tests/predicates/request/`):
- ✅ Method - tested
- ✅ Path - tested (53 lines)
- ✅ Header - tested (123 lines with Eq, Exist, In operations)
- ✅ Query - tested (58 lines with Eq, Exist, In operations)
- ✅ Body - tested (263 lines with Jq and ProtoBuf, Eq/Exist/In operations)

**Response Predicates** (in `hitbox-http/tests/predicates/reponse/`):
- ✅ StatusCode - tested (40 lines)
- ✅ Body - tested (207 lines)

**Conditional Predicates** (in `hitbox-http/tests/predicates/conditions.rs`):
- ✅ Not - tested (95 lines)
- ✅ Or - tested

**Extractors** (in `hitbox-http/tests/extractors/`):
- ✅ Method - tested
- ✅ Path - tested
- ✅ Header - tested
- ✅ Query - tested

### BDD Coverage Gaps

**Critical gaps** (unit tested but no BDD scenarios):
- ❌ Header predicates (Eq, Exist, In) - 0 scenarios
- ❌ Header extractors - 0 scenarios
- ❌ Body predicates with JQ - 0 scenarios
- ❌ Body predicates with ProtoBuf - 0 scenarios
- ❌ Path predicates (pattern matching) - 0 scenarios
- ❌ Query predicates (only extraction tested, not predicate matching) - 0 scenarios
- ❌ Response status code predicates - 0 scenarios (only tested via error-response.feature)
- ❌ Response body predicates - 0 scenarios
- ❌ Conditional predicates (Not, Or) in real scenarios - 0 scenarios
- ❌ Complex predicate combinations (And + Or trees) - 0 scenarios
- ❌ Stale cache comprehensive behavior - partially tested
- ❌ Multiple extractors combinations - 0 scenarios
- ❌ Edge cases: empty bodies, malformed JSON, missing headers - 0 scenarios
- ❌ Configuration defaults (MaybeUndefined behavior) - 0 scenarios
- ❌ Real-world scenarios (REST API, GraphQL, multi-tenant, auth) - 0 scenarios

---

## DETAILED IMPLEMENTATION PLAN

### PHASE 1: Core Request Predicates (HIGH PRIORITY)

#### 1.1 Header Predicates ⭐ CRITICAL
**File**: `tests/features/header-predicates.feature`

**Scenarios to implement**:

1. **Header Eq operation - exact match**
   ```gherkin
   Given hitbox with policy and request predicates
   When execute request with specific header value
   Then response is cached
   When execute request with different header value
   Then response is NOT cached
   ```

2. **Header Eq - case-insensitive header name**
   ```gherkin
   Given predicate matches "X-API-Key"
   When request has header "x-api-key"
   Then header matches (case-insensitive)
   ```

3. **Header Exist operation - presence check**
   ```gherkin
   Given predicate checks for Authorization header existence
   When request has Authorization header (any value)
   Then request is cacheable
   When request missing Authorization header
   Then request is non-cacheable
   ```

4. **Header In operation - value in list**
   ```gherkin
   Given predicate: Content-Type in ["application/json", "application/xml"]
   When request has Content-Type: application/json
   Then request is cacheable
   When request has Content-Type: text/html
   Then request is non-cacheable
   ```

5. **Missing headers - non-cacheable**
6. **Multiple headers with same name**
7. **Custom vs standard headers** (Content-Type, Authorization, User-Agent)
8. **Empty header values**
9. **Special characters in header values**
10. **Header extractor - cache key generation**

**Test data setup**:
```yaml
# Request predicates
- Header:
    x-api-key: "secret123"  # Eq operation
- Header:
    operation: Exist
    name: "Authorization"   # Exist operation
- Header:
    operation: In
    name: "Content-Type"
    values: ["application/json", "application/xml"]  # In operation
```

**Estimated scenarios**: 10 scenarios
**Priority**: CRITICAL - Headers are fundamental for API auth, tenant isolation, content negotiation
**Why first**: Zero BDD coverage despite unit tests existing

---

#### 1.2 Query Parameter Predicates
**File**: `tests/features/query-predicates.feature`

**Scenarios to implement**:

1. **Query Eq operation - exact match**
   ```gherkin
   Scenario: Query parameter exact match
     Given request predicates
       """yaml
       - Query:
           operation: Eq
           cache: "true"
       """
     When execute request with ?cache=true
     Then request is cacheable
     When execute request with ?cache=false
     Then request is non-cacheable
   ```

2. **Query Exist operation - parameter presence**
3. **Query In operation - value in list**
4. **Missing parameters - non-cacheable**
5. **URL encoding/decoding** (`?name=John%20Doe`)
6. **Empty parameter values** (`?param=`)
7. **Multiple values for same parameter** (`?tag=rust&tag=async`)
8. **Special characters in values**
9. **Array parameter values** (`?ids[]=1&ids[]=2`)

**Estimated scenarios**: 9 scenarios
**Priority**: HIGH - Query predicates control caching based on request parameters
**Dependency**: None

---

#### 1.3 Path Predicates
**File**: `tests/features/path-predicates.feature`

**Scenarios to implement**:

1. **Exact path match**
   ```gherkin
   Scenario: Exact path match
     Given request predicates
       """yaml
       - Path: /api/users
       """
     When execute request to /api/users
     Then request is cacheable
     When execute request to /api/posts
     Then request is non-cacheable
   ```

2. **Path with single parameter** (`/users/{id}`)
3. **Path with multiple parameters** (`/users/{uid}/posts/{pid}`)
4. **Non-matching paths - non-cacheable**
5. **URL encoding in paths** (`/users/John%20Doe`)
6. **Wildcard patterns** (`/api/*`)
7. **Path case sensitivity**
8. **Trailing slash handling** (`/api/users` vs `/api/users/`)

**Estimated scenarios**: 8 scenarios
**Priority**: HIGH - Path predicates are essential for endpoint-specific caching rules
**Dependency**: None

---

#### 1.4 Body Predicates with JQ ⭐ CRITICAL
**File**: `tests/features/body-predicates-jq.feature`

**Scenarios to implement**:

**JSON Body - Eq Operation**:

1. **Match string values at JQ path**
   ```gherkin
   Scenario: Body predicate matches string field
     Given request predicates
       """yaml
       - Body:
           parser: Jq
           jq: ".operation"
           operation:
             Eq: "getUser"
       """
     When execute request with body
       """json
       {"operation": "getUser", "userId": 123}
       """
     Then request is cacheable
     When execute request with body
       """json
       {"operation": "deleteUser", "userId": 123}
       """
     Then request is non-cacheable
   ```

2. **Match numeric values** (`.price == 100`)
3. **Match boolean values** (`.active == true`)
4. **Match null values** (`.deleted == null`)
5. **Match array values** (`.tags[0] == "rust"`)
6. **Match nested objects** (`.user.address.city == "NYC"`)
7. **Non-matching values - non-cacheable**
8. **Invalid JQ paths - non-cacheable**

**JSON Body - Exist Operation**:

9. **Path exists with value**
   ```gherkin
   Scenario: Body field existence check
     Given request predicates
       """yaml
       - Body:
           parser: Jq
           jq: ".operationName"
           operation: Exist
       """
     When execute request with body containing operationName
     Then request is cacheable
     When execute request with body missing operationName
     Then request is non-cacheable
   ```

10. **Path exists with null value** (`.field == null` vs field missing)
11. **Path does not exist - non-cacheable**
12. **Nested object path existence** (`.user.profile`)
13. **Array element existence** (`.[0]`)

**JSON Body - In Operation**:

14. **Value matches one item in list**
    ```gherkin
    Scenario: Body value in allowed list
      Given request predicates
        """yaml
        - Body:
            parser: Jq
            jq: ".method"
            operation:
              In: ["query", "mutation"]
        """
      When execute request with body {"method": "query"}
      Then request is cacheable
      When execute request with body {"method": "subscription"}
      Then request is non-cacheable
    ```

15. **Value matches multiple items**
16. **Value matches none - non-cacheable**
17. **Empty list behavior**
18. **Mixed type comparisons**

**Error Handling**:

19. **Invalid JSON body**
20. **Empty body with body predicate**

**Estimated scenarios**: 20 scenarios
**Priority**: CRITICAL - Body predicates enable GraphQL operation caching, API versioning, complex filtering
**Why fourth**: Zero BDD coverage despite comprehensive unit tests

---

### PHASE 2: Response Predicates (HIGH PRIORITY)

#### 2.1 Status Code Predicates
**File**: `tests/features/status-code-predicates.feature`

**Scenarios to implement**:

1. **Match 200 OK - cacheable**
   ```gherkin
   Scenario: Cache only 200 OK responses
     Given response predicates
       """yaml
       - Status: 200
       """
     When execute request returning 200
     Then response is cached
     When execute request returning 404
     Then response is NOT cached
   ```

2. **Match 201 Created**
3. **Match 204 No Content**
4. **Match 3xx redirects** (301, 302, 304)
5. **Non-matching status codes - non-cacheable**
6. **Multiple status codes with OR** (`200 OR 201 OR 204`)
7. **Error status codes explicit non-caching** (4xx, 5xx)
8. **Custom status code filtering**

**Estimated scenarios**: 8 scenarios
**Priority**: HIGH - Response status determines cache worthiness
**Note**: Partially covered in `error-response.feature` but needs dedicated coverage

---

#### 2.2 Response Body Predicates
**File**: `tests/features/response-body-predicates.feature`

**Scenarios to implement**:

1. **Match response JSON fields with JQ**
   ```gherkin
   Scenario: Cache responses without errors
     Given response predicates
       """yaml
       - Body:
           parser: Jq
           jq: ".error"
           operation: Exist
       """
     When execute request returning {"data": {...}, "error": null}
     Then response is NOT cached (error field exists even if null)
     When execute request returning {"data": {...}}
     Then response is cached (no error field)
   ```

2. **Check response structure existence**
3. **Conditional caching based on response content** (e.g., cache if `.success == true`)
4. **Cache responses with specific data shapes**
5. **Empty response bodies**
6. **Large response bodies**
7. **Invalid JSON responses - non-cacheable**
8. **Nested response data** (`.data.result`)
9. **Array responses** (`.[].status`)
10. **Response body with In operation** (`.status` in ["completed", "success"])

**Estimated scenarios**: 10 scenarios
**Priority**: MEDIUM - Enables conditional caching based on response content
**Use case**: Don't cache error responses with specific error codes

---

### PHASE 3: Conditional Predicates (MEDIUM PRIORITY)

#### 3.1 Not Predicate
**File**: `tests/features/conditional-not.feature`

**Scenarios to implement**:

1. **Invert method predicate - cache all except GET**
   ```gherkin
   Scenario: Cache all methods except GET
     Given request predicates
       """yaml
       Not:
         Method: GET
       """
     When execute GET request
     Then request is non-cacheable
     When execute POST request
     Then request is cacheable
   ```

2. **Invert path predicate** (cache all paths except `/admin/*`)
3. **Invert header predicate** (cache when header NOT present)
4. **Invert query predicate** (cache when param NOT equals value)
5. **Double negation** (`Not(Not(Method: GET))` = `Method: GET`)
6. **Not with complex predicates** (`Not(Or(...))`)

**Estimated scenarios**: 6 scenarios
**Priority**: MEDIUM - Essential for exclusion rules
**Use case**: "Cache everything except admin endpoints"

---

#### 3.2 Or Predicate
**File**: `tests/features/conditional-or.feature`

**Scenarios to implement**:

1. **Multiple method matching - GET OR HEAD**
   ```gherkin
   Scenario: Cache GET or HEAD requests
     Given request predicates
       """yaml
       Or:
         - Method: GET
         - Method: HEAD
       """
     When execute GET request
     Then request is cacheable
     When execute HEAD request
     Then request is cacheable
     When execute POST request
     Then request is non-cacheable
   ```

2. **Multiple path patterns** (`/api/users OR /api/posts`)
3. **Multiple header values** (`x-version: v1 OR x-version: v2`)
4. **Complex OR trees** (nested ORs)
5. **OR with mixed predicate types** (method OR path)
6. **All predicates fail - non-cacheable**
7. **OR with single predicate** (should behave same as single)
8. **Three or more OR branches**

**Estimated scenarios**: 8 scenarios
**Priority**: MEDIUM - Common for multiple method/endpoint caching rules

---

#### 3.3 Complex Predicate Combinations
**File**: `tests/features/predicate-combinations.feature`

**Scenarios to implement**:

1. **AND + OR combinations**
   ```gherkin
   Scenario: Method is GET AND (path is /api/users OR /api/posts)
     Given request predicates
       """yaml
       - Method: GET
       - Or:
           - Path: /api/users
           - Path: /api/posts
       """
     When execute GET /api/users
     Then request is cacheable
     When execute POST /api/users
     Then request is non-cacheable (method doesn't match)
     When execute GET /api/books
     Then request is non-cacheable (path doesn't match)
   ```

2. **NOT + OR combinations** (`NOT(POST OR DELETE)`)
3. **Deep nesting** (3+ levels)
4. **Request AND Response predicates together**
   ```gherkin
   Scenario: Cache GET requests that return 200
     Given request predicates [Method: GET]
     And response predicates [Status: 200]
     When execute GET request returning 200
     Then response is cached
     When execute GET request returning 404
     Then response is NOT cached
   ```

5. **Real-world API rules** (Cache GET to `/api/*` with status 200 and no error field)
6. **Multiple extractors with complex predicates**
7. **OR of complex conditions**
8. **NOT of complex conditions**
9. **Mixed request and response predicates with conditionals**
10. **Chained predicates** (method AND path AND header AND query)

**Estimated scenarios**: 10 scenarios
**Priority**: MEDIUM - Tests real-world complex caching rules

---

### PHASE 4: Extractor Combinations (MEDIUM PRIORITY)

#### 4.1 Multiple Extractors
**File**: `tests/features/extractor-combinations.feature`

**Scenarios to implement**:

1. **Method + Path extractors**
   ```gherkin
   Scenario: Cache key includes method and path parameters
     Given key extractors
       """yaml
       - Method:
       - Path: "/users/{id}"
       """
     When execute GET /users/123
     Then cache key is "method=GET:id=123"
     When execute POST /users/123
     Then cache key is "method=POST:id=123"
   ```

2. **Method + Path + Query extractors**
   ```gherkin
   Scenario: Cache key includes query parameters
     Given key extractors
       """yaml
       - Method:
       - Path: "/users/{id}"
       - Query: "version"
       """
     When execute GET /users/123?version=v1
     Then cache key is "method=GET:id=123:version=v1"
   ```

3. **Method + Path + Header extractors** (tenant ID, API version)
4. **All extractors combined** (method + path + query + header)
5. **Duplicate key part handling** (two extractors produce same key name)
6. **Order independence** (same extractors, different order = same key)
7. **Missing values - partial extraction** (query param not present)
8. **URL encoding in extracted values**
9. **Multiple query parameters extracted**
10. **Multiple path parameters extracted** (`/users/{uid}/posts/{pid}`)

**Estimated scenarios**: 10 scenarios
**Priority**: MEDIUM - Ensures cache keys are generated correctly
**Use case**: Multi-tenant apps, API versioning

---

### PHASE 5: Cache Mechanics (HIGH PRIORITY)

#### 5.1 Stale Cache Behavior ⭐ CRITICAL
**File**: `tests/features/stale-cache.feature`

**Scenarios to implement** (with mock time):

1. **Fresh → Stale transition - serve stale data**
   ```gherkin
   @serial
   Scenario: Cache serves stale data after TTL but before expiration
     Given mock time is enabled
     And hitbox with policy
       """yaml
       !Enabled
       ttl: 10
       stale: 20
       """
     When execute request at T=0
     Then response header "X-Cache-Status" is "MISS"
     When sleep 5 (T=5, within TTL)
     And execute same request
     Then response header "X-Cache-Status" is "HIT"
     When sleep 10 (T=15, past TTL, within stale)
     And execute same request
     Then response header "X-Cache-Status" is "STALE" or "HIT"
     And response body matches original
   ```

2. **Stale → Expired transition - fetch new data**
   ```gherkin
   When sleep 25 (T=40, past TTL+stale)
   And execute same request
   Then response header "X-Cache-Status" is "MISS"
   ```

3. **Stale-while-revalidate behavior** (if implemented)
4. **No stale configured** (direct TTL → Expired)
   ```gherkin
   Given hitbox with policy
     """yaml
     !Enabled
     ttl: 10
     # No stale configured
     """
   When sleep 11
   Then cache is expired (no stale period)
   ```

5. **Stale time longer than TTL**
6. **Edge case: stale=0** (immediately stale after TTL)
7. **Multiple requests during stale period**
8. **Stale cache with different cache keys** (ensure isolation)

**Estimated scenarios**: 8 scenarios
**Priority**: HIGH - Stale mechanics are critical for production use
**Note**: Partially tested in `mock-time-ttl.feature` but needs comprehensive coverage

---

#### 5.2 Cache Key Consistency
**File**: `tests/features/cache-key-generation.feature`

**Scenarios to implement**:

1. **Identical requests generate same key**
   ```gherkin
   Scenario: Identical requests use same cache entry
     Given key extractors [Method, Path, Query: "page"]
     When execute GET /users?page=1
     And execute GET /users?page=1
     Then both requests use same cache key
     And cache has 1 record
   ```

2. **Different requests generate different keys**
3. **Key part ordering consistency** (`method=GET:id=123` = `id=123:method=GET`?)
4. **Special characters in keys** (URL encoding)
5. **Key collision avoidance**
6. **Empty values in keys** (`?param=`)
7. **URL encoding normalization** (`John%20Doe` = `John Doe`?)
8. **Query parameter order independence** (`?a=1&b=2` = `?b=2&a=1`?)

**Estimated scenarios**: 8 scenarios
**Priority**: MEDIUM - Ensures cache correctness

---

#### 5.3 Cache Invalidation (if supported)
**File**: `tests/features/cache-invalidation.feature`

**Scenarios to implement**:

1. **Manual cache invalidation by key**
2. **TTL-based automatic invalidation** (already tested)
3. **Invalidate by pattern** (if supported)
4. **Backend restart/reconnect scenarios**
5. **Clear all cache**

**Estimated scenarios**: 5 scenarios
**Priority**: LOW - May not be supported yet

---

### PHASE 6: Configuration System (MEDIUM PRIORITY)

#### 6.1 Configuration Defaults
**File**: `tests/features/configuration-defaults.feature`

**Scenarios to implement**:

1. **MaybeUndefined::Undefined behavior - use defaults**
   ```gherkin
   Scenario: Missing policy uses default Enabled
     Given hitbox with no policy specified
     # Should default to Enabled { ttl: 5, stale: None }
     When execute request
     Then caching is enabled with default TTL
   ```

2. **MaybeUndefined::Null behavior - neutral predicates**
3. **MaybeUndefined::Value behavior - user-specified**
4. **Missing policy - default Enabled with ttl=5**
5. **Missing request predicates - default Method: GET**
6. **Missing response predicates - default Status: 200**
7. **Missing extractors - default Method + Path: "*"**
8. **Empty configuration file**

**Estimated scenarios**: 8 scenarios
**Priority**: MEDIUM - Ensures configuration system works correctly

---

#### 6.2 Configuration Validation
**File**: `tests/features/configuration-validation.feature`

**Scenarios to implement**:

1. **Invalid YAML syntax - graceful failure**
2. **Invalid policy values** (negative TTL)
3. **Invalid predicate types**
4. **Invalid JQ expressions**
5. **Missing required fields**
6. **Type mismatches**

**Estimated scenarios**: 6 scenarios
**Priority**: LOW - Edge case error handling

---

### PHASE 7: Real-World Scenarios (HIGH PRIORITY)

#### 7.1 REST API Caching ⭐ CRITICAL
**File**: `tests/features/rest-api-scenarios.feature`

**Scenarios to implement**:

1. **User CRUD operations**
   ```gherkin
   Scenario: GET requests are cached, POST/PUT/DELETE are not
     Given request predicates
       """yaml
       - Method: GET
       """
     When execute GET /users/123
     Then response is cached
     When execute POST /users
     Then response is NOT cached
     When execute PUT /users/123
     Then response is NOT cached
     When execute DELETE /users/123
     Then response is NOT cached
   ```

2. **Pagination with query parameters**
   ```gherkin
   Scenario: Different pages have different cache keys
     Given key extractors
       """yaml
       - Method:
       - Path: "/users"
       - Query: "page"
       - Query: "per_page"
       """
     When execute GET /users?page=1&per_page=10
     And execute GET /users?page=2&per_page=10
     Then cache has 2 distinct records
   ```

3. **Filtering and sorting** (query param based)
4. **Resource relationships** (`/users/{id}/posts`)
5. **API versioning - header based** (x-api-version)
6. **API versioning - path based** (`/v1/users`, `/v2/users`)
7. **Tenant isolation** (header-based tenant ID in cache key)
8. **List endpoint with filters** (`/users?role=admin&status=active`)
9. **Detail endpoint caching** (`/users/{id}`)
10. **Nested resource caching** (`/users/{uid}/posts/{pid}`)

**Estimated scenarios**: 10 scenarios
**Priority**: CRITICAL - Real-world validation
**Use case**: Standard REST API patterns

---

#### 7.2 GraphQL Caching
**File**: `tests/features/graphql-scenarios.feature`

**Scenarios to implement**:

1. **Query operations - cacheable based on operation name**
   ```gherkin
   Scenario: GraphQL queries are cached by operation name
     Given request predicates
       """yaml
       - Method: POST
       - Body:
           parser: Jq
           jq: ".operationName"
           operation: Exist
       """
     And key extractors
       """yaml
       - Method:
       - Body:
           parser: Jq
           jq: ".operationName"
       """
     When execute GraphQL query
       """json
       {
         "query": "query GetUser($id: ID!) { user(id: $id) { name } }",
         "operationName": "GetUser",
         "variables": {"id": "123"}
       }
       """
     Then response is cached with key containing "GetUser"
   ```

2. **Mutation operations - non-cacheable**
3. **Different queries to same endpoint** (different cache keys based on body)
4. **Query variables in cache key**
5. **Introspection queries - cacheable**
6. **Anonymous queries** (no operationName)
7. **Subscription operations - non-cacheable**

**Estimated scenarios**: 7 scenarios
**Priority**: MEDIUM - Common use case for modern APIs

---

#### 7.3 Multi-Tenant Applications ⭐ CRITICAL
**File**: `tests/features/multi-tenant-scenarios.feature`

**Scenarios to implement**:

1. **Tenant ID in header - different tenants = different cache keys**
   ```gherkin
   Scenario: Each tenant has isolated cache
     Given key extractors
       """yaml
       - Header: "x-tenant-id"
       - Path: "/users/{id}"
       """
     When tenant A executes GET /users/123
     Then cache key contains "x-tenant-id=tenant-a"
     When tenant B executes GET /users/123
     Then cache key contains "x-tenant-id=tenant-b"
     And cache has 2 records
   ```

2. **Tenant ID in path parameter** (`/tenants/{tenant}/users`)
3. **Tenant isolation verification** (tenant A can't access tenant B's cache)
4. **Missing tenant ID - non-cacheable or error**
5. **Default tenant handling**
6. **Tenant-specific TTL** (if supported)

**Estimated scenarios**: 6 scenarios
**Priority**: CRITICAL - Common pattern for SaaS applications

---

#### 7.4 Authentication & Authorization
**File**: `tests/features/auth-scenarios.feature`

**Scenarios to implement**:

1. **API key in header - included in cache key**
   ```gherkin
   Scenario: Different API keys get separate cache entries
     Given key extractors
       """yaml
       - Header: "x-api-key"
       - Path: "/data"
       """
     When execute request with x-api-key: key1
     Then cache key contains "x-api-key=key1"
     When execute request with x-api-key: key2
     Then cache key contains "x-api-key=key2"
     And cache has 2 records
   ```

2. **Bearer token - check presence but NOT in cache key** (security)
3. **User-specific caching** (user ID in header/token)
4. **Public vs authenticated endpoints** (different policies)
5. **Missing auth headers - non-cacheable for protected endpoints**
6. **Role-based caching** (admin vs user different cache)
7. **Session-based caching**

**Estimated scenarios**: 7 scenarios
**Priority**: MEDIUM - Security-sensitive caching patterns

---

### PHASE 8: Edge Cases & Error Handling (LOW-MEDIUM PRIORITY)

#### 8.1 Malformed Requests
**File**: `tests/features/error-handling.feature`

**Scenarios to implement**:

1. **Invalid JSON body - non-cacheable**
   ```gherkin
   Scenario: Malformed JSON body handled gracefully
     Given request predicates with body JQ
     When execute request with invalid JSON
     Then request is non-cacheable
     And no error is thrown
   ```

2. **Malformed URLs**
3. **Oversized request bodies**
4. **Invalid UTF-8 in headers**
5. **Missing required headers for extractors**
6. **Binary data in body** (ProtoBuf or error)
7. **Null bytes in strings**
8. **Very long header values**

**Estimated scenarios**: 8 scenarios
**Priority**: MEDIUM - Robust error handling

---

#### 8.2 Edge Cases
**File**: `tests/features/edge-cases.feature`

**Scenarios to implement**:

1. **Empty request body with body predicate**
2. **Empty response body**
3. **Very large bodies** (performance test)
4. **Concurrent identical requests** (should use same cache entry)
5. **Unicode and international characters** (paths/headers)
6. **Special characters in query parameters** (`?q=foo&bar`)
7. **Extremely long URLs**
8. **Zero TTL** (immediate expiration)
9. **Very large TTL** (years)

**Estimated scenarios**: 9 scenarios
**Priority**: LOW - Edge case coverage

---

### PHASE 9: Performance & Load (LOW PRIORITY)

#### 9.1 Performance Tests
**File**: `tests/features/performance.feature`

**Scenarios to implement**:

1. **Large number of predicates - no degradation**
2. **Large number of extractors**
3. **Complex JQ expressions** (deeply nested)
4. **Large cache sizes** (memory usage)
5. **Cache hit rate monitoring**
6. **Concurrent request handling**

**Note**: May require different test framework than Cucumber (e.g., criterion benchmarks).

**Estimated scenarios**: 6 scenarios
**Priority**: LOW - Performance validation

---

## PRIORITIZATION MATRIX

### Must-Have (Complete First) ⭐
1. **Header Predicates** (Phase 1.1) - CRITICAL - Zero BDD coverage
2. **Body Predicates with JQ** (Phase 1.4) - CRITICAL - Zero BDD coverage
3. **Status Code Predicates** (Phase 2.1) - HIGH - Partially covered
4. **Stale Cache Behavior** (Phase 5.1) - CRITICAL - Production use
5. **REST API Scenarios** (Phase 7.1) - CRITICAL - Real-world validation
6. **Multi-Tenant Scenarios** (Phase 7.3) - CRITICAL - Common use case

**Estimated**: ~62 scenarios, 3-4 weeks

### Should-Have (Complete Second) ⭐
7. Query Parameter Predicates (Phase 1.2)
8. Path Predicates (Phase 1.3)
9. Response Body Predicates (Phase 2.2)
10. Or Predicate (Phase 3.2)
11. Extractor Combinations (Phase 4.1)
12. Configuration Defaults (Phase 6.1)
13. Auth Scenarios (Phase 7.4)

**Estimated**: ~60 scenarios, 3-4 weeks

### Nice-to-Have (Complete Third)
14. Not Predicate (Phase 3.1)
15. Complex Predicate Combinations (Phase 3.3)
16. Cache Key Consistency (Phase 5.2)
17. GraphQL Scenarios (Phase 7.2)
18. Malformed Requests (Phase 8.1)

**Estimated**: ~41 scenarios, 2-3 weeks

### Optional (Complete If Time Permits)
19. Cache Invalidation (Phase 5.3)
20. Configuration Validation (Phase 6.2)
21. Edge Cases (Phase 8.2)
22. Performance Tests (Phase 9.1)

**Estimated**: ~26 scenarios, 1-2 weeks

---

## IMPLEMENTATION STATISTICS

### Total Scenarios by Phase

| Phase | Focus Area | Scenarios | Priority |
|-------|-----------|-----------|----------|
| 1.1 | Header Predicates | 10 | CRITICAL |
| 1.2 | Query Predicates | 9 | HIGH |
| 1.3 | Path Predicates | 8 | HIGH |
| 1.4 | Body Predicates (JQ) | 20 | CRITICAL |
| 2.1 | Status Code Predicates | 8 | HIGH |
| 2.2 | Response Body Predicates | 10 | MEDIUM |
| 3.1 | Not Predicate | 6 | MEDIUM |
| 3.2 | Or Predicate | 8 | MEDIUM |
| 3.3 | Predicate Combinations | 10 | MEDIUM |
| 4.1 | Extractor Combinations | 10 | MEDIUM |
| 5.1 | Stale Cache | 8 | CRITICAL |
| 5.2 | Cache Key Consistency | 8 | MEDIUM |
| 5.3 | Cache Invalidation | 5 | LOW |
| 6.1 | Configuration Defaults | 8 | MEDIUM |
| 6.2 | Configuration Validation | 6 | LOW |
| 7.1 | REST API | 10 | CRITICAL |
| 7.2 | GraphQL | 7 | MEDIUM |
| 7.3 | Multi-Tenant | 6 | CRITICAL |
| 7.4 | Auth | 7 | MEDIUM |
| 8.1 | Error Handling | 8 | MEDIUM |
| 8.2 | Edge Cases | 9 | LOW |
| 9.1 | Performance | 6 | LOW |
| **TOTAL** | | **187** | |

### Current vs Target Coverage

- **Current BDD scenarios**: ~20 scenarios
- **Target BDD scenarios**: ~187 scenarios
- **Gap**: ~167 scenarios to implement
- **Estimated effort**: 9-13 weeks

### Coverage by Priority

- **CRITICAL**: 54 scenarios (29%)
- **HIGH**: 25 scenarios (13%)
- **MEDIUM**: 82 scenarios (44%)
- **LOW**: 26 scenarios (14%)

---

## RECOMMENDED TIMELINE

### Week 1-2: Critical Gaps ⭐
**Goal**: Close the most critical BDD coverage gaps

- ✅ Phase 1.1: Header Predicates (10 scenarios)
- ✅ Phase 1.4: Body Predicates with JQ (20 scenarios)

**Deliverable**: 30 scenarios, 2 feature files

---

### Week 3-4: Response & Caching ⭐
**Goal**: Complete response predicates and stale cache testing

- ✅ Phase 2.1: Status Code Predicates (8 scenarios)
- ✅ Phase 5.1: Stale Cache Behavior (8 scenarios)

**Deliverable**: 16 scenarios, 2 feature files

---

### Week 5-6: Real-World Scenarios ⭐
**Goal**: Validate real-world use cases

- ✅ Phase 7.1: REST API Scenarios (10 scenarios)
- ✅ Phase 7.3: Multi-Tenant Scenarios (6 scenarios)

**Deliverable**: 16 scenarios, 2 feature files

---

### Week 7-8: Predicates & Extractors
**Goal**: Complete remaining request predicates and extractors

- ✅ Phase 1.2: Query Parameter Predicates (9 scenarios)
- ✅ Phase 1.3: Path Predicates (8 scenarios)
- ✅ Phase 4.1: Extractor Combinations (10 scenarios)

**Deliverable**: 27 scenarios, 3 feature files

---

### Week 9-10: Advanced Features
**Goal**: Conditional predicates and response body

- ✅ Phase 3.1: Not Predicate (6 scenarios)
- ✅ Phase 3.2: Or Predicate (8 scenarios)
- ✅ Phase 3.3: Predicate Combinations (10 scenarios)
- ✅ Phase 2.2: Response Body Predicates (10 scenarios)

**Deliverable**: 34 scenarios, 4 feature files

---

### Week 11: Configuration & Auth
**Goal**: Configuration system and authentication patterns

- ✅ Phase 6.1: Configuration Defaults (8 scenarios)
- ✅ Phase 7.4: Auth Scenarios (7 scenarios)

**Deliverable**: 15 scenarios, 2 feature files

---

### Week 12-13: Remaining Features (Optional)
**Goal**: Complete all remaining phases

- ✅ Phase 5.2: Cache Key Consistency (8 scenarios)
- ✅ Phase 7.2: GraphQL Scenarios (7 scenarios)
- ✅ Phase 8.1: Error Handling (8 scenarios)
- ✅ Phase 8.2: Edge Cases (9 scenarios)
- ⚠️ Phase 5.3: Cache Invalidation (5 scenarios, if supported)
- ⚠️ Phase 6.2: Configuration Validation (6 scenarios)
- ⚠️ Phase 9.1: Performance Tests (6 scenarios)

**Deliverable**: 49 scenarios, 7 feature files

---

## QUICK START IMPLEMENTATION

### Recommended First Feature to Implement

**File**: `tests/features/header-predicates.feature`

**Why**:
- Zero BDD coverage despite unit tests
- Critical for authentication/authorization patterns
- Common in real-world APIs (API keys, tenant IDs, versioning)
- Relatively straightforward to implement (3 operations: Eq, Exist, In)

**Sample Implementation**:

```gherkin
Feature: Header Predicate Functionality

  @integration
  Scenario: Header Eq operation - exact match
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```
    And request predicates
      ```yaml
      - Header:
          x-api-key: "secret123"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Headers]
      x-api-key: secret123
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Headers]
      x-api-key: secret123
      ```
    Then response header "X-Cache-Status" is "HIT"

  @integration
  Scenario: Header Eq operation - different value not cached
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```
    And request predicates
      ```yaml
      - Header:
          x-api-key: "secret123"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Headers]
      x-api-key: wrongkey
      ```
    Then response status is 200
    And response headers have no "X-Cache-Status" header
    And cache has 0 records

  @integration
  Scenario: Header Exist operation - presence check
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```
    And request predicates
      ```yaml
      - Header:
          operation: Exist
          name: Authorization
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Headers]
      Authorization: Bearer any-token-here
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @integration
  Scenario: Header Exist operation - missing header not cached
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```
    And request predicates
      ```yaml
      - Header:
          operation: Exist
          name: Authorization
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      ```
    Then response status is 200
    And response headers have no "X-Cache-Status" header
    And cache has 0 records

  @integration
  Scenario: Header In operation - value in list
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```
    And request predicates
      ```yaml
      - Header:
          operation: In
          name: Content-Type
          values:
            - application/json
            - application/xml
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Headers]
      Content-Type: application/json
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Headers]
      Content-Type: application/xml
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 2 records

  @integration
  Scenario: Header In operation - value not in list
    Given hitbox with policy
      ```yaml
      !Enabled
      ttl: 10
      ```
    And request predicates
      ```yaml
      - Header:
          operation: In
          name: Content-Type
          values:
            - application/json
            - application/xml
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      [Headers]
      Content-Type: text/html
      ```
    Then response status is 200
    And response headers have no "X-Cache-Status" header
    And cache has 0 records
```

---

## APPENDIX: Test Data Setup

### Sample Test Database

**File**: `hitbox-test/data.yaml`

Current test data includes:
- Authors: Robert Sheckley
- Books: "Victim Prime" and others
- Endpoints:
  - `GET /v1/authors/{author_id}/books` - List books (paginated)
  - `GET /v1/authors/{author_id}/books/{book_id}` - Get book details

### Additional Test Data Needed

For comprehensive BDD testing, consider adding:

1. **Multiple tenants** (for multi-tenant scenarios)
2. **Users with roles** (for auth scenarios)
3. **API versions** (for versioning scenarios)
4. **Error scenarios** (invalid IDs, missing resources)

---

## APPENDIX: CI/CD Integration

### Running BDD Tests

```bash
# Run all integration tests
cargo test --test integration

# Run specific feature
cargo test --test integration -- --tags @integration

# Run with specific concurrency (for @serial tests)
cargo test --test integration -- --concurrency 1

# Run specific scenario
cargo test --test integration -- "Header Eq operation"
```

### Test Organization

All BDD feature files should be in:
```
hitbox-test/tests/features/
├── header-predicates.feature
├── query-predicates.feature
├── path-predicates.feature
├── body-predicates-jq.feature
├── status-code-predicates.feature
├── response-body-predicates.feature
├── conditional-not.feature
├── conditional-or.feature
├── predicate-combinations.feature
├── extractor-combinations.feature
├── stale-cache.feature
├── cache-key-generation.feature
├── configuration-defaults.feature
├── rest-api-scenarios.feature
├── graphql-scenarios.feature
├── multi-tenant-scenarios.feature
├── auth-scenarios.feature
├── error-handling.feature
└── edge-cases.feature
```

---

## APPENDIX: Step Implementation Guidelines

### When Adding New Steps

If new Cucumber steps are needed beyond the current set, add them to:

- **Given steps**: `src/steps/given.rs`
- **When steps**: `src/steps/when.rs`
- **Then steps**: `src/steps/then.rs`

**Example**: Adding a step to check cache state:

```rust
// In src/steps/then.rs
#[then(expr = "cache entry for key {string} is in state {string}")]
async fn check_cache_entry_state(
    world: &mut HitboxWorld,
    key_pattern: String,
    expected_state: String,
) -> Result<(), Error> {
    // Parse key pattern
    let key_value_pairs: Vec<(&str, &str)> = key_pattern
        .split(':')
        .filter_map(|part| {
            let mut split = part.split('=');
            Some((split.next()?, split.next()?))
        })
        .collect();

    let cache_key = CacheKey::from_slice(&key_value_pairs);

    // Get cache entry and check state
    let entry = world.backend.cache.get(&cache_key).await
        .ok_or_else(|| anyhow!("Cache key not found"))?;

    let state = match entry.state(&MockTimeProvider::now()) {
        CacheState::Fresh => "fresh",
        CacheState::Stale => "stale",
        CacheState::Expired => "expired",
    };

    if state != expected_state {
        return Err(anyhow!(
            "Expected cache state '{}', found '{}'",
            expected_state,
            state
        ));
    }

    Ok(())
}
```

---

## CONCLUSION

This plan provides a comprehensive roadmap for implementing BDD tests for the Hitbox caching framework. By following the phased approach and prioritization matrix, you can:

1. **Close critical gaps** (headers, body predicates, stale cache)
2. **Validate real-world scenarios** (REST APIs, multi-tenant, auth)
3. **Ensure comprehensive coverage** (all predicates, extractors, configurations)
4. **Build confidence** for production deployments

The recommended timeline of 9-13 weeks covers all 187 scenarios, with the first 6 weeks focusing on the most critical Must-Have and Should-Have features.

**Next Steps**:
1. Review and approve this plan
2. Start with Phase 1.1: Header Predicates (10 scenarios)
3. Iterate through phases according to priority
4. Update this document as new requirements emerge

---

**Document Version**: 1.0
**Last Updated**: 2025-10-14
**Maintained By**: Hitbox Test Team
