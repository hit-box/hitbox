# Test Coverage Checklist for Predicates and Extractors

## Predicates

### Request Predicates

#### Method Predicate

##### Operations

###### Eq (Exact Match)
- [x] Match method request
- [x] Non-matching method returns non-cacheable

###### In (Multiple Methods)
- [x] Method in allowed list cached
- [x] Empty list behavior

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

#### Path Predicate

##### Operations

###### Eq (Exact Match)
- [x] Exact path match cached
- [x] Partial path match not cached
- [x] Path with path parameter cached
- [x] Path with trailing slash doesn't match path without trailing slash
- [x] Path case sensitivity
- [x] Path with encoded characters

#### Query Predicate

##### Operations

###### Eq (Exact Match)
- [x] Single query parameter match
- [x] Multiple query parameters match
- [x] Query parameter value mismatch not cached
- [x] Empty query parameter value

###### Exist (Presence Check)
- [x] Query parameter exists cached
- [x] Query parameter missing not cached

###### In (Multiple Values)
- [x] Value in list cached
- [x] Value not in list not cached
- [x] Empty list behavior

#### Body Predicate

##### Operations

###### Eq (Exact Match)
- [x] Simple field exact match cached
- [x] Field value mismatch not cached
- [x] Nested object field match cached
- [x] Array element match cached
- [x] Multiple body predicates - all must match
- [x] Missing field not cached
- [x] String value match
- [x] Number value match
- [x] Boolean value match
- [x] Null value match
- [x] Complex jq expression match
- [x] Missing field equals null

###### Exist (Presence Check)
- [x] Field exists cached (any value)
- [x] Different values for same field cache hit
- [x] Missing field not cached
- [x] Nested field existence
- [x] Array element existence

###### In (Multiple Values)
- [x] Value in list cached with same key
- [x] Different values in list share cache
- [x] Value not in list not cached
- [x] Empty list behavior
- [x] Multiple data types in list

##### Parsing Types
- [x] JSON with jq expressions (primary focus)
- [ ] ProtoBuf parsing (future work)

### Logical Predicates (Conditional Operators)

#### And Predicate (Implicit Chaining)

##### Basic Functionality
- [x] Two predicates both match - request cached
- [x] First predicate matches, second doesn't - request not cached
- [x] First predicate doesn't match - short-circuit, request not cached
- [x] Both predicates don't match - request not cached

##### Multiple Predicate Combinations
- [x] Three predicates all match - request cached (Method AND Header AND Path)
- [x] Method AND Header AND Query all match
- [x] Method AND Path AND Body all match
- [x] Four or more predicates chained together

##### Edge Cases
- [ ] Empty And list behavior (should be neutral/cacheable)
- [ ] Single predicate in And (behaves like that predicate)
- [ ] And with all neutral predicates
- [ ] Nested And within And

#### Or Predicate (Explicit Alternative Matching)

##### Basic Functionality
- [x] Left predicate matches - request cached
- [x] Right predicate matches - request cached
- [x] Both predicates match - request cached
- [x] Neither predicate matches - request not cached
- [x] Base predicate fails - request not cached regardless of Or branches

##### Multiple Or Branches
- [x] Three predicates in Or - any match caches (Method in GET,HEAD,OPTIONS)
- [x] Four or more predicates in Or
- [x] Or with different predicate types (Method OR Path OR Header)

##### Edge Cases
- [ ] Empty Or list behavior (should be non-cacheable)
- [ ] Single predicate in Or (behaves like that predicate)
- [ ] Or with all failing predicates
- [ ] Or with neutral predicates
- [ ] Nested Or within Or

##### Short-Circuit Behavior
- [ ] Left predicate matches - right predicate not evaluated
- [ ] Base predicate fails - Or branches not evaluated

#### Not Predicate (Negation/Inversion)

##### Basic Functionality
- [x] Wrapped predicate matches - result is NonCacheable
- [x] Wrapped predicate doesn't match - result is Cacheable

##### Double Negation
- [ ] Not(Not(predicate)) equals predicate
- [ ] Triple negation works correctly

##### Edge Cases
- [ ] Not with neutral predicate
- [ ] Not with always-failing predicate becomes always-caching
- [ ] Not with complex nested predicates

#### Complex Predicate Combinations

##### And + Or Combinations
- [x] (Method=GET OR Method=HEAD) AND Path=/api/*
- [x] Method=GET AND (Path=/users OR Path=/posts)
- [x] (Header(x-tenant-id=a) OR Header(x-tenant-id=b)) AND Method=GET
- [x] Method=GET AND (Header(Content-Type=json) OR Header(Content-Type=xml))

##### Not + Or Combinations
- [x] Not(Method=POST OR Method=DELETE) - caches GET,HEAD,PUT,PATCH,etc
- [x] Not(Path=/admin OR Path=/internal) - excludes multiple paths
- [x] Method=GET AND Not(Path OR Query)
- [ ] Not(Header(x-debug=true)) AND Method in [GET,HEAD]

##### Not + And Combinations
- [x] Not(Method=POST AND Path=/admin/*) - DeMorgan's law application
- [x] Not(Header(x-tenant-id exists) AND Method=DELETE)
- [x] Method=GET AND Not(Header(x-no-cache=true) AND Path=/dynamic/*)

##### Three-Level Nesting
- [x] And(Or(And(...))) - complex nested logic
- [x] Or(And(Not(...))) - mixed logical operators
- [x] Real-world: (Method=GET OR Method=HEAD) AND Not(Path=/admin/*) AND Header(x-api-key exists)
- [x] GraphQL advanced: Method=POST AND Body(.operationName exists) AND Not(Body(.operationName == "IntrospectionQuery")) AND Or(Header(x-tenant-id=a) OR Header(x-tenant-id=b))

##### Performance & Correctness
- [ ] Deep nesting (5+ levels) doesn't cause stack overflow
- [ ] Large Or branches (10+ alternatives) evaluated correctly
- [ ] Mixed predicate types in complex trees
- [ ] Short-circuit evaluation verified (base predicate fails early)

#### Logical Predicate Edge Cases

##### Empty Collections
- [ ] And([]) behavior (neutral/pass-through)
- [ ] Or([]) behavior (always non-cacheable)
- [ ] Nested empty operators

##### Degenerate Cases
- [ ] Single-predicate And (equivalent to that predicate)
- [ ] Single-predicate Or (equivalent to that predicate)
- [ ] Not(Not(Not(predicate))) - odd number of negations

##### Type Mixing
- [ ] And combining all predicate types (Method, Path, Header, Query, Body)
- [ ] Or with heterogeneous predicate types
- [ ] Not wrapping Or wrapping And wrapping predicates

##### Configuration Edge Cases
- [ ] YAML parsing of deeply nested logical structures
- [ ] Malformed logical predicate configuration
- [ ] Circular or recursive predicate references (should not be possible)

### Response Predicates

#### Status Predicate

##### Operations

###### Eq (Exact Match)
- [x] Status code exact match cached
- [x] Different status code not cached

###### In (Multiple Status Codes)
- [x] Status in list cached
- [x] Status not in list not cached
- [x] Empty list behavior
- [x] Single status in list
- [x] Multiple status codes (2-4 codes)

###### Class (Status Code Class)
- [x] Success class (2xx) cached
- [x] ClientError class (4xx) cached
- [x] Redirect class (3xx)
- [x] ServerError class (5xx)

## Cache Key Extractors

### Request Extractors

#### Body Extractor

##### Basic Functionality
- [x] Extract JSON field from request body for cache key
- [ ] Extract nested JSON field
- [ ] Extract array element
- [ ] Extract multiple fields
- [x] String values wrapped in single quotes
- [ ] Null values handling
- [x] Simple field extraction (.title)
- [ ] Array index (.items[0])
- [ ] Array filter (.items[] | select(.active))
- [ ] Boolean jq expressions

#### Header Extractor

##### Basic Functionality
- [x] Extract header value for cache key
- [ ] Multiple header extractors
- [ ] Case-insensitive header names
- [ ] Missing header (no cache key part)
- [ ] Header with empty value
- [ ] Multiple header values (comma-separated)
- [ ] Header with special characters
- [ ] Header value trimming

#### Method Extractor

##### Basic Functionality
- [x] Extract HTTP method for cache key

#### Path Extractor

##### Basic Functionality
- [x] Extract path parameters for cache key
- [ ] Wildcard patterns
- [ ] Missing path parameters
- [ ] URL-encoded path parameters
- [ ] Non-matching path patterns

#### Query Extractor

##### Basic Functionality
- [x] Extract query parameter for cache key
- [ ] Multiple query parameters
- [ ] Query parameter with array values
- [ ] Missing query parameter (no cache key part)
- [ ] Query parameter with empty value
- [ ] Query parameter with special characters
- [ ] URL-encoded query values
- [ ] Multiple values for same parameter

### Extractor Combinations

#### Multiple Extractors
- [ ] Method + Path extractors
- [ ] Method + Query extractors
- [ ] Header + Body extractors
- [ ] All extractors combined

#### Extractor Order
- [ ] Extractor order affects cache key
- [ ] Same extractors different order produce same key

### Extractor Edge Cases

#### Configuration
- [ ] Empty extractor list (default behavior)
- [ ] Single extractor
- [ ] Many extractors (10+)
