# Test Coverage Checklist for Response Predicates

## Response Predicates

### Response Status Predicate

#### Operations

##### Eq (Exact Match)
- [x] Exact status code match cached
- [x] Status code mismatch not cached
- [x] Multiple status predicates - all must match

##### In (Multiple Status Codes)
- [x] Status code in list cached
- [x] Status code not in list not cached
- [x] Empty status list behavior
- [x] Single status code in list
- [x] Multiple status codes in list (2-4 codes)

##### Range (Status Code Range)
- [x] Status in range cached
- [x] Status outside range not cached
- [x] Range lower boundary (inclusive)
- [x] Range upper boundary (inclusive)
- [x] Single-value range (start equals end)
- [x] Invalid range (start > end) validation (fails at configuration parse time)

##### Class (HTTP Status Class)
- [x] Success class (2xx) cached
- [x] Redirect class (3xx) behavior
- [x] Client error class (4xx) cached
- [x] Server error class (5xx) behavior
- [x] Informational class (1xx) behavior

#### Explicit Syntax Support
- [x] Eq operation with explicit syntax `{ eq: value }`
- [x] In operation with explicit syntax `{ in: [values] }`
- [x] Range operation with explicit syntax `{ range: [start, end] }` (required - no implicit form)
- [x] Class operation with explicit syntax `{ class: ClassName }`

#### Notes
- **Range** operation requires explicit syntax only: `{ range: [start, end] }`
- **In**, **Eq**, and **Class** support both explicit and implicit syntax
- Implicit array syntax `[values]` always resolves to **In** operation (not Range)
- **Range validation**: Invalid ranges (start > end) are rejected at configuration parse time with clear error messages
- Unit tests for range validation are in `hitbox-configuration/tests/test_reponse.rs`

### Response Header Predicate

#### Operations

##### Eq (Exact Match)
- [x] Exact header value match cached
- [x] Different header value not cached
- [x] Case-insensitive header name
- [x] Multiple header predicates - all must match
- [x] Header with empty value
- [x] Multiple header values with EQ operation
- [x] Header value case sensitivity
- [x] Header with whitespace trimmed
- [x] Header missing not cached

##### Exist (Presence Check)
- [x] Header exists cached
- [x] Header missing not cached
- [x] Additional header doesn't affect cache decision
- [x] Header with any value matches
- [x] Case-insensitive header name in Exist

##### In (Multiple Values)
- [x] Value in list cached
- [x] Value not in list not cached
- [x] Multiple header values with IN operation
- [x] Single value in list
- [x] Empty list behavior
- [x] Header value case sensitivity with IN operation
- [x] Header missing not cached

##### Contains (Substring Match)
- [x] Header value contains substring cached
- [x] Header value doesn't contain substring not cached
- [x] Case-sensitive substring matching
- [x] Multiple headers with contains
- [x] Header missing not cached

##### Regex (Pattern Match)
- [x] Header value matches regex pattern cached
- [x] Header value doesn't match pattern not cached
- [x] Complex regex patterns
- [x] Multiple headers with regex
- [x] Header missing not cached
- [x] Case-sensitive regex matching
- [x] Invalid regex pattern handled at configuration parse time (unit test: `test_invalid_regex_pattern_rejected`)

#### Notes
- **Invalid regex validation**: Similar to range validation for status predicates, invalid regex patterns are tested at the unit test level in `hitbox-configuration/tests/test_reponse.rs`
- **Unit test coverage**: Contains, Regex deserialization, and validation tests added (`test_response_header_contains_deserialize`, `test_response_header_regex_deserialize`, `test_invalid_regex_pattern_rejected`, `test_valid_regex_pattern_accepted`)
- **BDD test coverage**: 32 scenarios covering all runtime behavior for Eq, Exist, In, Contains, and Regex operations
