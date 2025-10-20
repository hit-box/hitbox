# Test Coverage Checklist for Predicates and Extractors

## Predicates

### Request Predicates

#### Method Predicate

##### Operations

###### Eq (Exact Match)
- [x] Match method request
- [ ] Non-matching method returns non-cacheable

###### In (Multiple Methods)
- [x] Method in allowed list cached
- [ ] Multiple methods in list (3+ methods)
- [ ] Empty list behavior

#### Path Predicate

##### Operations

###### Eq (Exact Match)
- [ ] Exact path match cached
- [ ] Partial path match not cached
- [ ] Path with trailing slash
- [ ] Path without trailing slash
- [ ] Path case sensitivity
- [ ] Path with special characters
- [ ] Path with encoded characters
- [ ] Empty path
- [ ] Root path "/"

#### Query Predicate

##### Operations

###### Eq (Exact Match)
- [x] Single query parameter match
- [ ] Multiple query parameters match
- [ ] Query parameter value mismatch not cached
- [ ] Query parameter name mismatch not cached
- [ ] Empty query parameter value
- [ ] Query parameter with special characters
- [ ] Query parameter with encoded characters
- [ ] Case sensitivity of parameter names
- [ ] Case sensitivity of parameter values

###### Exist (Presence Check)
- [ ] Query parameter exists cached
- [ ] Query parameter missing not cached
- [ ] Query parameter with any value

###### In (Multiple Values)
- [x] Value in list cached
- [ ] Value not in list not cached
- [ ] Multiple values in list
- [ ] Single value in list
- [ ] Empty list behavior

#### Header Predicate

##### Operations

###### Eq (Exact Match)
- [x] Exact header value match cached
- [x] Different header value not cached
- [x] Case-insensitive header name
- [x] Multiple header predicates - all must match
- [x] Header with empty value
- [x] Multiple header values with EQ operation
- [x] Header value case sensitivity
- [x] Header with whitespace trimmed
- [x] Header missing not cached

###### Exist (Presence Check)
- [x] Header exists cached
- [x] Header missing not cached
- [x] Additional header doesn't affect cache decision
- [x] Header with any value matches
- [x] Case-insensitive header name in Exist

###### In (Multiple Values)
- [x] Value in list cached
- [x] Value not in list not cached
- [x] Multiple header values with IN operation
- [x] Single value in list
- [x] Empty list behavior
- [x] Header value case sensitivity with IN operation
- [x] Header missing not cached

#### Logical Operations

##### And Operation
- [x] All predicates match cached
- [ ] First predicate fails not cached
- [ ] Middle predicate fails not cached
- [ ] Last predicate fails not cached
- [ ] Single predicate in And
- [ ] Empty And operation
- [ ] Nested And operations
- [ ] Mixed predicate types (Method + Path + Query + Header)

##### Or Operation
- [x] First predicate matches cached
- [x] Middle predicate matches cached
- [x] Last predicate matches cached
- [x] No predicates match not cached
- [x] Single predicate matching
- [x] Single predicate not matching
- [x] Mixed predicate types
- [x] Or expression tree serialization
- [ ] Empty Or operation
- [ ] Nested Or operations
- [ ] Or with all predicates matching (multiple matches)

##### Not Operation
> **Note:** NOT operation exists in `hitbox-http` but is not yet exposed in configuration layer. These tests are pending implementation.
- [ ] Not(Method) - negates method predicate
- [ ] Not(Path) - negates path predicate
- [ ] Not(Query) - negates query predicate
- [ ] Not(Header) - negates header predicate
- [ ] Not(And(...)) - negates AND operation
- [ ] Not(Or(...)) - negates OR operation
- [ ] Not(Not(...)) - double negation

##### Mixed Operations (And + Or + Not)
- [x] Or(Query, Method, And(Method, Path)) serialization
- [x] Or(Query, And(Method, Path)) serialization
- [ ] And(Or(...), Or(...))
- [ ] Or(And(...), And(...))
- [ ] Not(And(Or(...), Or(...)))
- [ ] And(Not(...), Not(...))
- [ ] Or(Not(...), ...)
- [ ] Deeply nested operations (3+ levels)
- [ ] Complex predicate tree evaluation

#### Expression Format

##### Flat Format
- [x] Flat format deserialization
- [ ] Flat format with single predicate
- [ ] Flat format with multiple predicates
- [ ] Empty flat format

##### Tree Format
- [x] Tree format with operations
- [ ] Tree format with single predicate
- [ ] Tree format with nested operations

### Response Predicates

#### Status Predicate

##### Operations

###### Eq (Exact Match)
- [ ] Status 200 cached
- [ ] Status 201 cached
- [ ] Status 204 cached
- [ ] Status 301 cached
- [ ] Status 302 cached
- [x] Status 404 not cached
- [x] Status 500 not cached
- [ ] Status 502 not cached
- [ ] Status 503 not cached
- [ ] Custom status codes (e.g., 418)

#### Logical Operations

##### And Operation
- [ ] Multiple status predicates with And
- [ ] Nested And operations

##### Or Operation
- [ ] Multiple status predicates with Or
- [ ] Nested Or operations
- [ ] Or with first matching
- [ ] Or with last matching
- [ ] Or with no matching

##### Not Operation
> **Note:** NOT operation exists in `hitbox-http` but is not yet exposed in configuration layer. These tests are pending implementation.
- [ ] Not(Status) - negates status predicate
- [ ] Not(And(...)) - negates AND operation
- [ ] Not(Or(...)) - negates OR operation

## Extractors

### Path Extractor
- [x] Path template serialization
- [x] Path parameter extraction in cache key
- [ ] Multiple path parameters
- [ ] Path parameter at start
- [ ] Path parameter at end
- [ ] Path parameter in middle
- [ ] Adjacent path parameters
- [ ] Path without parameters
- [ ] Path with special characters in parameters
- [ ] Path with encoded characters in parameters
- [ ] Path parameter matching greedy vs non-greedy

### Method Extractor
- [x] Method serialization
- [x] Method extraction in cache key
- [ ] Different methods generate different keys
- [ ] Method extraction with all HTTP methods

### Query Extractor
- [x] Query parameter generates distinct cache keys
- [ ] Multiple query parameters in key
- [ ] Query parameter order independence
- [ ] Query parameter with empty value
- [ ] Query parameter missing
- [ ] Query parameter with special characters
- [ ] Query parameter with encoded characters
- [ ] Multiple values for same query parameter

## Integration Tests

### Cache Policy
- [x] Disabled cache policy - no storage
- [x] Enabled cache policy - store and retrieve
- [x] TTL expiration
- [ ] Stale cache behavior
- [ ] Cache with stale-while-revalidate
- [ ] Cache invalidation
- [ ] Cache size limits
- [ ] Cache key collision handling

### Combined Scenarios
- [ ] Multiple predicates with extractors
- [ ] Complex predicate tree with extractors
- [ ] Different cache keys for same path with different query params
- [ ] Different cache keys for same path with different methods
- [ ] Same cache key for different paths (edge case)

---

## Summary Statistics

- **Total Test Cases:** 173 (includes 17 NOT operation tests pending implementation)
- **Implemented (Unit Tests):** 19
- **Implemented (BDD Tests):** 35
- **Total Implemented:** 54
- **Coverage:** ~31.2% (54/173)
- **Pending NOT Operation:** 17 tests (needs configuration layer support)

## Priority Areas for Testing

1. **NOT Operation** - Exists in code but not exposed in configuration layer (17 tests pending)
2. **Path Predicate** - No BDD tests currently
3. **Query Predicate Exist & In operations** - Minimal coverage
4. **Response Predicates** - Only error cases covered
5. **Path Extractor** - Only basic serialization tested
6. **Query Extractor** - Only single parameter tested
7. **Nested logical operations** - Limited coverage
8. **Edge cases** - Special characters, encoding, empty values
