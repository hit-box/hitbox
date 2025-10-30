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
- [ ] Large status code list (10+ codes)

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
