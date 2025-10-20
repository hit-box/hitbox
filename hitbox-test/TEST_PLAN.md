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
