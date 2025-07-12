# Hitbox Test Cases

## Request Predicates 

### Method
- [ ] GET - Basic implementation exists
- [ ] POST - Basic implementation exists
- [ ] PUT - Basic implementation exists
- [ ] PATCH - Basic implementation exists
- [ ] DELETE - Basic implementation exists
- [ ] HEAD - Test coverage needed
- [ ] OPTIONS - Test coverage needed
- [ ] TRACE - Test coverage needed
- [ ] CONNECT - Test coverage needed
- [ ] Custom methods - Test edge cases

### Path
- [ ] Static paths: "", "/", "/home", "/home/", "/Home"
- [ ] Parameterized: "/user/{id}", "/user/{user_id}/post/{post_id}"
- [ ] Wildcard
- [ ] Multiple wildcards
- [ ] Case sensitivity: "/User" vs "/user" 
- [ ] Trailing slashes: "/path/" vs "/path"

### Headers
- [ ] Eq - Exact value matching
- [ ] Exist - Header presence check
- [ ] In - Value in list matching
- [ ] Case-insensitive header names
- [ ] Multiple header values (comma-separated)
- [ ] Header value patterns/regex
- [ ] Standard headers: Content-Type, Authorization, User-Agent
- [ ] Custom headers: X-Custom-Header, X-API-Key
- [ ] Missing headers (negative tests)
- [ ] Empty header values

### Query Parameters
- [ ] Eq - Exact parameter matching
- [ ] Exist - Parameter presence check
- [ ] In - Value in list matching
- [ ] Multiple values for same parameter: "?tag=red&tag=blue"
- [ ] Array notation: "?items[]=1&items[]=2"
- [ ] Nested parameters: "?user[name]=john&user[age]=25"
- [ ] URL encoding: "?name=john%20doe"
- [ ] Special characters: "?search=hello+world&filter=price>100"
- [ ] Empty parameters: "?param=" vs "?param"
- [ ] Parameter ordering independence
- [ ] Case sensitivity

### Body

#### JSON
- [ ] Eq - Exact JSON value matching
- [ ] Exist - JSON path existence
- [ ] In - JSON value in list
- [ ] Nested JSON structures
- [ ] Large JSON payloads (>1MB)
- [ ] Malformed JSON
- [ ] Empty JSON: {} vs []
- [ ] JSON with special characters
- [ ] JQ expressions: .users[0].name
- [ ] Complex JQ queries: .users | select(.age > 18)

#### Protobuf (binary data)
- [ ] Eq - Exact protobuf message matching
- [ ] Exist - Field existence check
- [ ] In - Field value in list
- [ ] Nested protobuf messages
- [ ] Repeated fields
- [ ] Optional fields
- [ ] Oneof fields
- [ ] Protobuf enums
- [ ] Protobuf maps
- [ ] Different protobuf versions
- [ ] Invalid protobuf data
- [ ] Large protobuf messages
- [ ] Protobuf field paths: .user.profile.name

#### Other Body Types
- [ ] Plain text body
- [ ] Binary data (images, files)
- [ ] Streaming bodies
- [ ] Compressed bodies (gzip, deflate)
- [ ] Empty body vs no body

## Request Expressions

### Or Expressions
- [ ] First node: A || B
- [ ] Nested: (A || B) || C
- [ ] Complex nesting: (A || B) && (C || D)
- [ ] Multiple OR conditions: A || B || C || D

### Not Expressions
- [ ] First node: !A
- [ ] Nested: !(A || B)
- [ ] Double negation: !!A
- [ ] De Morgan's laws: !(A && B) === (!A || !B)

## Response Predicates

### Status Code
- [ ] Exact status: 200, 404, 500

### Headers
- [ ] Eq - Exact value matching
- [ ] Exist - Header presence check
- [ ] In - Value in list matching
- [ ] Case-insensitive header names
- [ ] Multiple header values (comma-separated)
- [ ] Header value patterns/regex
- [ ] Standard headers: Content-Type, Authorization, User-Agent
- [ ] Custom headers: X-Custom-Header, X-API-Key
- [ ] Missing headers (negative tests)
- [ ] Empty header values

### Body

#### JSON
- [ ] Eq - Exact JSON value matching
- [ ] Exist - JSON path existence
- [ ] In - JSON value in list
- [ ] Nested JSON structures
- [ ] Large JSON payloads (>1MB)
- [ ] Malformed JSON
- [ ] Empty JSON: {} vs []
- [ ] JSON with special characters
- [ ] JQ expressions: .users[0].name
- [ ] Complex JQ queries: .users | select(.age > 18)

#### Protobuf (binary data)
- [ ] Eq - Exact protobuf message matching
- [ ] Exist - Field existence check
- [ ] In - Field value in list
- [ ] Nested protobuf messages
- [ ] Repeated fields
- [ ] Optional fields
- [ ] Oneof fields
- [ ] Protobuf enums
- [ ] Protobuf maps
- [ ] Different protobuf versions
- [ ] Invalid protobuf data
- [ ] Large protobuf messages
- [ ] Protobuf field paths: .user.profile.name

#### Other Body Types
- [ ] Plain text body
- [ ] Binary data (images, files)
- [ ] Streaming bodies
- [ ] Compressed bodies (gzip, deflate)
- [ ] Empty body vs no body


## Response Expressions

### Or Expressions
- [ ] First node: A || B
- [ ] Nested: (A || B) || C
- [ ] Complex nesting: (A || B) && (C || D)
- [ ] Multiple OR conditions: A || B || C || D

### Not Expressions
- [ ] First node: !A
- [ ] Nested: !(A || B)
- [ ] Double negation: !!A
- [ ] De Morgan's laws: !(A && B) === (!A || !B)

## Extractors (Cache Key Generation)

### Method Extractor
- [ ] Extract HTTP method for cache key
- [ ] Method normalization (case handling)

### Path Extractor
- [ ] Extract path parameters: /user/{id} -> id=123
- [ ] Path normalization: /path/ -> /path
- [ ] Unicode path handling

### Headers Extractor
- [ ] Extract specific header values
- [ ] Multiple header extraction

### Query Extractor
- [ ] Extract query parameter values
- [ ] Query parameter sorting for consistent keys

### Body Extractor
- [ ] Extract values from request body
- [ ] JSON path extraction: .user.id
- [ ] Protobuf field extraction

## Advanced Testing Scenarios

### Cache Policy Testing
- [ ] TTL (Time To Live) behavior
- [ ] Stale cache handling
- [ ] Cache eviction policies
- [ ] Cache size limitations
- [ ] Cache hit/miss ratios

### Backend Integration
- [ ] Redis backend testing
- [ ] Moka (in-memory) backend testing
- [ ] Backend failover scenarios
- [ ] Backend performance testing
- [ ] Backend connection handling

### Concurrency & Performance
- [ ] Concurrent request handling
- [ ] Race condition testing
- [ ] Load testing scenarios
- [ ] Memory usage testing
- [ ] Latency measurements

### Error Handling
- [ ] Malformed request handling
- [ ] Backend connection failures
- [ ] Cache corruption scenarios
- [ ] Timeout handling
- [ ] Resource exhaustion

### Security Testing
- [ ] Cache poisoning prevention
- [ ] Authorization header handling
- [ ] Sensitive data exclusion
- [ ] Request validation
- [ ] DoS attack prevention

### Integration Testing
- [ ] Axum framework integration
- [ ] Tower middleware integration
- [ ] HTTP client compatibility
- [ ] Metrics and monitoring
- [ ] Configuration management

### Edge Cases
- [ ] Very large requests (>10MB)
- [ ] Very small requests (empty)
- [ ] Unusual HTTP methods
- [ ] Malformed headers
- [ ] Invalid UTF-8 sequences
- [ ] Network interruptions
- [ ] System resource constraints

## Framework-Specific Testing

### Axum Integration
- [ ] Axum extractors compatibility
- [ ] Axum middleware ordering
- [ ] Axum error handling
- [ ] Axum streaming responses

### Tower Integration
- [ ] Tower service composition
- [ ] Tower middleware stack
- [ ] Tower error handling
- [ ] Tower metrics integration

### HTTP Client Testing
- [ ] Different HTTP client libraries
- [ ] HTTP/1.1 vs HTTP/2
- [ ] Keep-alive connections
- [ ] Connection pooling

## Regression Testing
- [ ] Backwards compatibility
- [ ] API stability
- [ ] Performance regression detection
- [ ] Memory leak detection
- [ ] Configuration migration
