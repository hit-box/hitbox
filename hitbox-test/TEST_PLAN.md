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
- [ ] Three predicates all match - request cached (Method AND Header AND Path)
- [ ] Method AND Header AND Query all match
- [ ] Method AND Path AND Body all match
- [ ] Four or more predicates chained together

##### Real-World Scenarios
- [ ] Authentication check: Method=GET AND Header(Authorization exists)
- [ ] Tenant isolation: Header(x-tenant-id exists) AND Path matches pattern
- [ ] API versioning: Header(x-api-version=v1) AND Method=GET
- [ ] Content-type filtering: Method=POST AND Header(Content-Type=application/json)
- [ ] Multi-tenant API: Header(x-tenant-id) AND Header(x-api-key) AND Method in [GET,HEAD]
- [ ] GraphQL query filtering: Method=POST AND Body(.operationName exists) AND Body(.operationName != "IntrospectionQuery")

##### Edge Cases
- [ ] Empty And list behavior (should be neutral/cacheable)
- [ ] Single predicate in And (behaves like that predicate)
- [ ] And with all neutral predicates
- [ ] Nested And within And

#### Or Predicate (Explicit Alternative Matching)

##### Real-World Use Cases
- Multi-method endpoints: Cache GET OR HEAD requests
- Content negotiation: Accept JSON OR XML responses
- Multiple authentication methods: API key OR Bearer token
- Development vs Production: Cache in production OR when cache-debug header present
- Health check patterns: Path=/health OR Path=/healthz OR Path=/ready

##### Basic Functionality
- [x] Left predicate matches - request cached
- [x] Right predicate matches - request cached
- [x] Both predicates match - request cached
- [x] Neither predicate matches - request not cached
- [x] Base predicate fails - request not cached regardless of Or branches

##### Multiple Or Branches
- [ ] Three predicates in Or - any match caches (Method in GET,HEAD,OPTIONS)
- [ ] Four or more predicates in Or
- [ ] Or with different predicate types (Method OR Path OR Header)

##### Real-World Scenarios
- [ ] Safe methods: Method=GET OR Method=HEAD
- [ ] CORS preflight: Method=OPTIONS OR Method=HEAD
- [ ] Content types: Header(Accept=application/json) OR Header(Accept=application/xml)
- [ ] Multi-path caching: Path=/api/users OR Path=/api/posts
- [ ] API version compatibility: Header(x-api-version=v1) OR Header(x-api-version=v2)
- [ ] Public OR authenticated: Path=/public/* OR Header(Authorization exists)
- [ ] Multiple tenant support: Header(x-tenant-id=tenant-a) OR Header(x-tenant-id=tenant-b)

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

##### Real-World Use Cases
- Exclusion patterns: Cache everything EXCEPT admin endpoints
- Method exclusion: Cache all methods EXCEPT POST,PUT,DELETE
- Health check filtering: Don't cache monitoring endpoints
- Development mode: Cache only when NOT in debug mode
- Bot filtering: Cache only when NOT a bot (User-Agent check)

##### Basic Functionality
- [x] Wrapped predicate matches - result is NonCacheable
- [x] Wrapped predicate doesn't match - result is Cacheable

##### Real-World Scenarios
- [ ] Exclude mutations: Not(Method=POST)
- [ ] Exclude admin: Not(Path=/admin/*)
- [ ] Exclude health checks: Not(Path=/health)
- [ ] Cache non-authenticated only: Not(Header(Authorization exists))
- [ ] Exclude specific tenant: Not(Header(x-tenant-id=excluded-tenant))
- [ ] Exclude monitoring: Not(Or(Path=/metrics OR Path=/health OR Path=/ready))
- [ ] Cache when no debug flag: Not(Header(x-debug=true))

##### Double Negation
- [ ] Not(Not(predicate)) equals predicate
- [ ] Triple negation works correctly

##### Edge Cases
- [ ] Not with neutral predicate
- [ ] Not with always-failing predicate becomes always-caching
- [ ] Not with complex nested predicates

#### Complex Predicate Combinations

##### And + Or Combinations
- [ ] (Method=GET OR Method=HEAD) AND Path=/api/*
- [ ] Method=GET AND (Path=/users OR Path=/posts)
- [ ] (Header(x-tenant-id=a) OR Header(x-tenant-id=b)) AND Method=GET
- [ ] Method in [GET,HEAD] AND (Header(Accept=json) OR Header(Accept=xml))

##### Not + Or Combinations
- [ ] Not(Method=POST OR Method=DELETE) - caches GET,HEAD,PUT,PATCH,etc
- [ ] Not(Path=/admin OR Path=/internal) - excludes multiple paths
- [ ] Method=GET AND Not(Path=/health OR Path=/metrics)
- [ ] Not(Header(x-debug=true)) AND Method in [GET,HEAD]

##### Not + And Combinations
- [ ] Not(Method=POST AND Path=/admin/*) - DeMorgan's law application
- [ ] Not(Header(x-tenant-id exists) AND Method=DELETE)
- [ ] Method=GET AND Not(Header(x-no-cache=true) AND Path=/dynamic/*)

##### Three-Level Nesting
- [ ] And(Or(And(...))) - complex nested logic
- [ ] Or(And(Not(...))) - mixed logical operators
- [ ] Real-world: (Method=GET OR Method=HEAD) AND Not(Path=/admin/*) AND Header(x-api-key exists)
- [ ] GraphQL advanced: Method=POST AND Body(.operationName exists) AND Not(Body(.operationName == "IntrospectionQuery")) AND Or(Header(x-tenant-id=a) OR Header(x-tenant-id=b))

##### Request + Response Predicate Combinations
- [ ] Request: Method=GET AND Response: Status=200
- [ ] Request: Not(Method=POST) AND Response: Status in [200,201,204]
- [ ] Request: Or(Method=GET, Method=HEAD) AND Response: Not(Status in [4xx,5xx])

##### Practical Multi-Condition Scenarios
- [ ] REST API: Method=GET AND Path=/api/* AND Header(Authorization exists) AND Not(Query(cache=false))
- [ ] Multi-tenant SaaS: Header(x-tenant-id exists) AND Method in [GET,HEAD] AND Not(Path=/admin/*) AND (Header(x-api-version=v1) OR Header(x-api-version=v2))
- [ ] GraphQL with auth: Method=POST AND Body(.query exists) AND Header(Authorization exists) AND Not(Body(.operationName == "Mutation"))
- [ ] Public API with rate limiting: Or(Header(x-api-tier=premium), Header(x-api-tier=enterprise)) AND Method=GET AND Not(Header(x-no-cache exists))
- [ ] Development environment: (Header(x-environment=production) OR Header(x-cache-enabled=true)) AND Method=GET AND Not(Path=/debug/*)

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
