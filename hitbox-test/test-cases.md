# Hitbox HTTP Caching Framework - Test Cases

This document provides a comprehensive checklist of all functionality that needs to be tested for predicates and extractors in the hitbox-http crate.

## Request Predicates

### Method Predicate (`method.rs`)
- [ ] Match GET requests
- [ ] Match POST requests  
- [ ] Match PUT requests
- [ ] Match DELETE requests
- [ ] Match PATCH requests
- [ ] Match HEAD requests
- [ ] Match OPTIONS requests
- [ ] Match CONNECT requests
- [ ] Match TRACE requests
- [ ] Non-matching method returns non-cacheable
- [ ] Chaining with other predicates

### Path Predicate (`path.rs`)
- [ ] Exact path match (e.g., `/api/users`)
- [ ] Path with single parameter (e.g., `/users/{id}`)
- [ ] Path with multiple parameters (e.g., `/users/{id}/posts/{post_id}`)
- [ ] Path with optional segments
- [ ] Path with wildcard patterns
- [ ] Non-matching path returns non-cacheable
- [ ] Case sensitivity handling
- [ ] URL encoding/decoding in paths
- [ ] Chaining with other predicates

### Header Predicate (`header.rs`)

#### Header Operations - Eq (Exact Match)
- [ ] Match exact header value (case-sensitive)
- [ ] Match exact header value (case-insensitive for header name)
- [ ] Non-matching header value returns non-cacheable
- [ ] Missing header returns non-cacheable
- [ ] Multiple values for same header (comma-separated)
- [ ] Custom headers
- [ ] Standard headers (Content-Type, Authorization, etc.)

#### Header Operations - Exist (Header Existence)
- [ ] Header exists (any value)
- [ ] Header does not exist returns non-cacheable
- [ ] Empty header value but header exists
- [ ] Multiple headers with same name

#### Header Operations - In (Value in List)
- [ ] Header value matches one item in list
- [ ] Header value matches multiple items in list
- [ ] Header value matches no items in list (non-cacheable)
- [ ] Empty list behavior
- [ ] Case sensitivity in list matching

### Query Predicate (`query.rs`)

#### Query Operations - Eq (Exact Match)
- [ ] Match exact query parameter value
- [ ] Non-matching parameter value returns non-cacheable
- [ ] Missing parameter returns non-cacheable
- [ ] URL encoding/decoding in values
- [ ] Special characters in parameter values
- [ ] Empty parameter value
- [ ] Multiple values for same parameter

#### Query Operations - Exist (Parameter Existence)
- [ ] Parameter exists (any value)
- [ ] Parameter does not exist returns non-cacheable
- [ ] Empty parameter value but parameter exists
- [ ] Multiple parameters with same name

#### Query Operations - In (Value in List)
- [ ] Parameter value matches one item in list
- [ ] Parameter value matches multiple items in list
- [ ] Parameter value matches no items in list (non-cacheable)
- [ ] Empty list behavior
- [ ] Array parameter values

### Body Predicate (`body.rs`)

#### JSON Body with JQ Parsing
##### Body Operations - Eq (Exact Match)
- [ ] Match exact JSON value at JQ path
- [ ] Match string values
- [ ] Match numeric values
- [ ] Match boolean values
- [ ] Match null values
- [ ] Match array values
- [ ] Match object values
- [ ] Non-matching value returns non-cacheable
- [ ] Invalid JQ path returns non-cacheable

##### Body Operations - Exist (Value Existence)
- [ ] JSON path exists (any value)
- [ ] JSON path does not exist returns non-cacheable
- [ ] Nested object path existence
- [ ] Array element existence
- [ ] Null value at existing path

##### Body Operations - In (Value in List)
- [ ] JSON value matches one item in list
- [ ] JSON value matches multiple items in list
- [ ] JSON value matches no items in list (non-cacheable)
- [ ] Empty list behavior
- [ ] Mixed type comparisons

#### Protocol Buffer Body Parsing
##### Body Operations - Eq (Exact Match)
- [ ] Match exact protobuf field value
- [ ] Match string fields
- [ ] Match numeric fields
- [ ] Match boolean fields
- [ ] Match enum fields
- [ ] Match repeated fields
- [ ] Match nested message fields
- [ ] Non-matching value returns non-cacheable
- [ ] Invalid field path returns non-cacheable

##### Body Operations - Exist (Field Existence)
- [ ] Protobuf field exists (any value)
- [ ] Protobuf field does not exist returns non-cacheable
- [ ] Optional field handling
- [ ] Required field handling
- [ ] Default value handling

##### Body Operations - In (Value in List)
- [ ] Protobuf value matches one item in list
- [ ] Protobuf value matches multiple items in list
- [ ] Protobuf value matches no items in list (non-cacheable)
- [ ] Empty list behavior

#### Error Handling
- [ ] Invalid JSON body
- [ ] Invalid protobuf body
- [ ] Empty body
- [ ] Large body handling
- [ ] Binary data in body
- [ ] Malformed JQ expressions
- [ ] Invalid message descriptor for protobuf

## Response Predicates

### Status Code Predicate (`status_code.rs`)
- [ ] Match 200 OK
- [ ] Match 201 Created
- [ ] Match 204 No Content
- [ ] Match 400 Bad Request
- [ ] Match 401 Unauthorized
- [ ] Match 403 Forbidden
- [ ] Match 404 Not Found
- [ ] Match 500 Internal Server Error
- [ ] Match custom status codes
- [ ] Non-matching status code returns non-cacheable
- [ ] Chaining with other response predicates

### Response Body Predicate (`body.rs`)
*Same test cases as Request Body Predicate above*

#### JSON Response Body with JQ Parsing
##### Body Operations - Eq (Exact Match)
- [ ] Match exact JSON value at JQ path
- [ ] Match string values
- [ ] Match numeric values
- [ ] Match boolean values
- [ ] Match null values
- [ ] Match array values
- [ ] Match object values

##### Body Operations - Exist (Value Existence)
- [ ] JSON path exists (any value)
- [ ] JSON path does not exist returns non-cacheable
- [ ] Nested object path existence
- [ ] Array element existence

##### Body Operations - In (Value in List)
- [ ] JSON value matches one item in list
- [ ] JSON value matches multiple items in list
- [ ] JSON value matches no items in list (non-cacheable)

#### Protocol Buffer Response Body
*Same test cases as Request Body Protobuf tests*

## Request Extractors

### Method Extractor (`method.rs`)
- [ ] Extract GET method
- [ ] Extract POST method
- [ ] Extract PUT method
- [ ] Extract DELETE method
- [ ] Extract PATCH method
- [ ] Extract HEAD method
- [ ] Extract OPTIONS method
- [ ] Extract custom methods
- [ ] Key part name is "method"
- [ ] Key part value matches HTTP method string
- [ ] Chaining with other extractors

### Path Extractor (`path.rs`)
- [ ] Extract single path parameter (e.g., `/users/{id}`)
- [ ] Extract multiple path parameters (e.g., `/users/{user_id}/posts/{post_id}`)
- [ ] Extract parameters with different types (string, numeric)
- [ ] Handle URL encoding in extracted values
- [ ] Handle special characters in path parameters
- [ ] Non-matching path pattern extracts nothing
- [ ] Empty path parameters
- [ ] Key part names match parameter names
- [ ] Key part values match extracted values
- [ ] Chaining with other extractors

### Header Extractor (`header.rs`)
- [ ] Extract existing header value
- [ ] Extract standard headers (Content-Type, Authorization, User-Agent)
- [ ] Extract custom headers
- [ ] Handle missing headers (no key part generated)
- [ ] Handle headers with multiple values
- [ ] Handle empty header values
- [ ] Case-insensitive header name matching
- [ ] Key part name matches header name
- [ ] Key part value matches header value
- [ ] Chaining with other extractors

### Query Extractor (`query.rs`)
- [ ] Extract single query parameter value
- [ ] Extract multiple values for same parameter
- [ ] Extract parameters with special characters
- [ ] Extract URL-encoded parameter values
- [ ] Handle missing parameters (no key part generated)
- [ ] Handle empty parameter values
- [ ] Extract array-style parameters (e.g., `ids[]=1&ids[]=2`)
- [ ] Key part name matches parameter name
- [ ] Key part values match parameter values
- [ ] Chaining with other extractors

## Conditional Predicates

### Not Predicate (`not.rs`)
- [ ] Invert cacheable to non-cacheable
- [ ] Invert non-cacheable to cacheable
- [ ] Work with method predicates
- [ ] Work with path predicates
- [ ] Work with header predicates
- [ ] Work with query predicates
- [ ] Work with body predicates
- [ ] Work with response predicates
- [ ] Chaining multiple not operations
- [ ] Complex nested conditions

### Or Predicate (`or.rs`)
- [ ] Left predicate cacheable, right predicate cacheable → cacheable
- [ ] Left predicate cacheable, right predicate non-cacheable → cacheable
- [ ] Left predicate non-cacheable, right predicate cacheable → cacheable
- [ ] Left predicate non-cacheable, right predicate non-cacheable → non-cacheable
- [ ] Work with method predicates
- [ ] Work with path predicates
- [ ] Work with header predicates
- [ ] Work with query predicates
- [ ] Work with body predicates
- [ ] Work with response predicates
- [ ] Chaining multiple or operations
- [ ] Complex nested conditions

## Integration Tests

### Predicate Chaining
- [ ] Method + Path predicates
- [ ] Method + Header predicates
- [ ] Method + Query predicates
- [ ] Path + Header predicates
- [ ] Path + Query predicates
- [ ] Header + Query predicates
- [ ] Request + Response predicates
- [ ] Complex multi-predicate chains
- [ ] Conditional predicates with chaining

### Extractor Chaining
- [ ] Method + Path extractors
- [ ] Method + Header extractors
- [ ] Method + Query extractors
- [ ] Path + Header extractors
- [ ] Path + Query extractors
- [ ] Header + Query extractors
- [ ] All extractors combined
- [ ] Duplicate key part names handling

### Real-World Scenarios
- [ ] REST API endpoint with path parameters
- [ ] GraphQL endpoint with query operations
- [ ] API with authentication headers
- [ ] File upload endpoints
- [ ] JSON API with complex request bodies
- [ ] Microservice communication with protobuf
- [ ] Pagination with query parameters
- [ ] Content negotiation with Accept headers
- [ ] API versioning through headers or paths
- [ ] Multi-tenant applications with tenant identification

### Error Handling and Edge Cases
- [ ] Malformed HTTP requests
- [ ] Very large request/response bodies
- [ ] Binary data handling
- [ ] Unicode and international characters
- [ ] Concurrent request processing
- [ ] Memory usage with large predicates/extractors
- [ ] Performance with complex JQ expressions
- [ ] Invalid protobuf schema handling

### Cache Key Generation
- [ ] Consistent key generation for identical requests
- [ ] Different keys for different requests
- [ ] Key collision avoidance
- [ ] Key length optimization
- [ ] Special character handling in keys
- [ ] Key part ordering consistency

## Performance Tests
- [ ] Large number of predicates performance
- [ ] Large number of extractors performance
- [ ] Complex JQ expression performance
- [ ] Large protobuf message parsing performance
- [ ] Memory usage under load
- [ ] Concurrent predicate evaluation
- [ ] Cache key generation performance