# Hitbox BDD Test Plan - Overview

**Generated**: 2025-10-14
**Version**: 1.0

---

## Executive Summary

Comprehensive BDD test plan for Hitbox HTTP caching framework covering **187 scenarios** across **9 phases**.

**Current Coverage**: 6 feature files, ~20 scenarios
**Target Coverage**: 22 feature files, ~187 scenarios
**Gap**: ~167 scenarios to implement
**Estimated Effort**: 9-13 weeks

---

## Project Context

**Hitbox** is an asynchronous HTTP caching framework built on Tower middleware for Rust, providing:
- Flexible predicates for cache filtering (method, path, headers, query, body with JQ)
- Smart extractors for cache key generation
- TTL and stale cache mechanics with time-mocking support
- YAML-based configuration system
- Backend-agnostic architecture (Moka, Redis)

### Current BDD Coverage

**Tested** (6 feature files):
- ✅ Cache policies (Enabled/Disabled, TTL)
- ✅ Method predicates (GET, HEAD)
- ✅ Query parameter extraction
- ✅ Error response handling (404, 500)
- ✅ Mock time for TTL testing
- ✅ Basic stale cache behavior

**Critical Gaps**:
- ❌ Header predicates and extractors
- ❌ Body predicates with JQ
- ❌ Path predicates
- ❌ Query predicates (filtering)
- ❌ Response predicates (status, body)
- ❌ Conditional predicates (Not, Or)
- ❌ Complex predicate combinations
- ❌ Comprehensive stale cache testing
- ❌ Real-world scenarios (REST API, multi-tenant, auth, GraphQL)

---

## Implementation Phases

### PHASE 1: Core Request Predicates (47 scenarios)

**File**: Request filtering capabilities

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Header Predicates | `header-predicates.feature` | 10 | ⭐ CRITICAL |
| Query Predicates | `query-predicates.feature` | 9 | HIGH |
| Path Predicates | `path-predicates.feature` | 8 | HIGH |
| Body Predicates (JQ) | `body-predicates-jq.feature` | 20 | ⭐ CRITICAL |

**Coverage**:
- Header operations: Eq, Exist, In
- Query operations: Eq, Exist, In
- Path pattern matching with parameters
- Body JQ filtering: Eq, Exist, In for JSON fields
- Edge cases: missing values, encoding, special characters

---

### PHASE 2: Response Predicates (18 scenarios)

**File**: Response-based caching decisions

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Status Code Predicates | `status-code-predicates.feature` | 8 | HIGH |
| Response Body Predicates | `response-body-predicates.feature` | 10 | MEDIUM |

**Coverage**:
- Status code matching (200, 201, 204, 3xx, 4xx, 5xx)
- Response body JQ filtering
- Conditional caching based on response content
- Error response handling

---

### PHASE 3: Conditional Predicates (24 scenarios)

**File**: Logical combinations and complex rules

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Not Predicate | `conditional-not.feature` | 6 | MEDIUM |
| Or Predicate | `conditional-or.feature` | 8 | MEDIUM |
| Complex Combinations | `predicate-combinations.feature` | 10 | MEDIUM |

**Coverage**:
- Negation logic (cache everything except...)
- OR logic (multiple methods, paths, headers)
- AND + OR combinations
- Deep nesting (3+ levels)
- Request AND Response predicates together

---

### PHASE 4: Extractor Combinations (10 scenarios)

**File**: Cache key generation with multiple extractors

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Multiple Extractors | `extractor-combinations.feature` | 10 | MEDIUM |

**Coverage**:
- Method + Path + Query + Header combinations
- Order independence
- Missing values handling
- URL encoding normalization
- Duplicate key part handling

---

### PHASE 5: Cache Mechanics (21 scenarios)

**File**: Cache behavior and lifecycle

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Stale Cache Behavior | `stale-cache.feature` | 8 | ⭐ CRITICAL |
| Cache Key Consistency | `cache-key-generation.feature` | 8 | MEDIUM |
| Cache Invalidation | `cache-invalidation.feature` | 5 | LOW |

**Coverage**:
- Fresh → Stale → Expired transitions
- Stale-while-revalidate
- Edge cases: stale=0, no stale configured
- Cache key collision avoidance
- Identical requests generate same key
- Manual/automatic invalidation

---

### PHASE 6: Configuration System (14 scenarios)

**File**: YAML configuration edge cases

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Configuration Defaults | `configuration-defaults.feature` | 8 | MEDIUM |
| Configuration Validation | `configuration-validation.feature` | 6 | LOW |

**Coverage**:
- MaybeUndefined behavior (undefined/null/value)
- Default policies and predicates
- Missing configuration handling
- Invalid YAML/values
- Type mismatches

---

### PHASE 7: Real-World Scenarios (30 scenarios)

**File**: End-to-end use cases

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| REST API Caching | `rest-api-scenarios.feature` | 10 | ⭐ CRITICAL |
| GraphQL Caching | `graphql-scenarios.feature` | 7 | MEDIUM |
| Multi-Tenant Apps | `multi-tenant-scenarios.feature` | 6 | ⭐ CRITICAL |
| Auth & Authorization | `auth-scenarios.feature` | 7 | MEDIUM |

**Coverage**:
- CRUD operations (GET cacheable, POST/PUT/DELETE not)
- Pagination and filtering
- API versioning (header/path based)
- Tenant isolation
- GraphQL queries vs mutations
- API keys, bearer tokens, user-specific caching

---

### PHASE 8: Edge Cases & Error Handling (17 scenarios)

**File**: Robustness testing

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Malformed Requests | `error-handling.feature` | 8 | MEDIUM |
| Edge Cases | `edge-cases.feature` | 9 | LOW |

**Coverage**:
- Invalid JSON body
- Malformed URLs
- Oversized bodies
- Invalid UTF-8
- Binary data
- Concurrent requests
- Unicode/special characters

---

### PHASE 9: Performance & Load (6 scenarios)

**File**: Performance validation

| Feature | File | Scenarios | Priority |
|---------|------|-----------|----------|
| Performance Tests | `performance.feature` | 6 | LOW |

**Coverage**:
- Large number of predicates/extractors
- Complex JQ expressions
- Large cache sizes
- Concurrent request handling

**Note**: May require different test framework (e.g., criterion benchmarks)

---

## Prioritization Matrix

### ⭐ Must-Have (Complete First)
**62 scenarios | 3-4 weeks**

1. Header Predicates (10) - Zero BDD coverage
2. Body Predicates with JQ (20) - Zero BDD coverage
3. Status Code Predicates (8) - Partially covered
4. Stale Cache Behavior (8) - Critical for production
5. REST API Scenarios (10) - Real-world validation
6. Multi-Tenant Scenarios (6) - Common use case

### Should-Have (Complete Second)
**60 scenarios | 3-4 weeks**

7. Query Parameter Predicates (9)
8. Path Predicates (8)
9. Response Body Predicates (10)
10. Or Predicate (8)
11. Extractor Combinations (10)
12. Configuration Defaults (8)
13. Auth Scenarios (7)

### Nice-to-Have (Complete Third)
**41 scenarios | 2-3 weeks**

14. Not Predicate (6)
15. Complex Predicate Combinations (10)
16. Cache Key Consistency (8)
17. GraphQL Scenarios (7)
18. Malformed Requests (8)

### Optional (Complete If Time Permits)
**26 scenarios | 1-2 weeks**

19. Cache Invalidation (5)
20. Configuration Validation (6)
21. Edge Cases (9)
22. Performance Tests (6)

---

## Statistics

### Coverage by Priority

| Priority | Scenarios | Percentage | Weeks |
|----------|-----------|------------|-------|
| CRITICAL | 54 | 29% | 3-4 |
| HIGH | 25 | 13% | 1-2 |
| MEDIUM | 82 | 44% | 4-5 |
| LOW | 26 | 14% | 1-2 |
| **TOTAL** | **187** | **100%** | **9-13** |

### Coverage by Phase

| Phase | Focus | Scenarios | Files |
|-------|-------|-----------|-------|
| 1 | Request Predicates | 47 | 4 |
| 2 | Response Predicates | 18 | 2 |
| 3 | Conditional Predicates | 24 | 3 |
| 4 | Extractor Combinations | 10 | 1 |
| 5 | Cache Mechanics | 21 | 3 |
| 6 | Configuration | 14 | 2 |
| 7 | Real-World Scenarios | 30 | 4 |
| 8 | Edge Cases | 17 | 2 |
| 9 | Performance | 6 | 1 |
| **TOTAL** | | **187** | **22** |

### Current vs Target

- **Current**: 6 feature files, ~20 scenarios
- **Target**: 22 feature files, ~187 scenarios
- **Gap**: 16 new files, ~167 scenarios
- **Increase**: 9.35x scenario count

---

## Recommended Timeline

### Weeks 1-2: Critical Gaps ⭐
- Header Predicates (10)
- Body Predicates with JQ (20)
- **Deliverable**: 30 scenarios, 2 files

### Weeks 3-4: Response & Caching ⭐
- Status Code Predicates (8)
- Stale Cache Behavior (8)
- **Deliverable**: 16 scenarios, 2 files

### Weeks 5-6: Real-World Scenarios ⭐
- REST API Scenarios (10)
- Multi-Tenant Scenarios (6)
- **Deliverable**: 16 scenarios, 2 files

**⭐ Must-Have Complete: 62 scenarios in 6 weeks**

---

### Weeks 7-8: Predicates & Extractors
- Query Parameter Predicates (9)
- Path Predicates (8)
- Extractor Combinations (10)
- **Deliverable**: 27 scenarios, 3 files

### Weeks 9-10: Advanced Features
- Not Predicate (6)
- Or Predicate (8)
- Predicate Combinations (10)
- Response Body Predicates (10)
- **Deliverable**: 34 scenarios, 4 files

### Week 11: Configuration & Auth
- Configuration Defaults (8)
- Auth Scenarios (7)
- **Deliverable**: 15 scenarios, 2 files

**Should-Have Complete: 138 scenarios in 11 weeks**

---

### Weeks 12-13: Remaining Features (Optional)
- Cache Key Consistency (8)
- GraphQL Scenarios (7)
- Error Handling (8)
- Edge Cases (9)
- Cache Invalidation (5, if supported)
- Configuration Validation (6)
- Performance Tests (6)
- **Deliverable**: 49 scenarios, 7 files

**All Features Complete: 187 scenarios in 13 weeks**

---

## Quick Start

### First Feature to Implement
**File**: `tests/features/header-predicates.feature`

**Why**:
- Zero BDD coverage despite unit tests
- Critical for auth/authorization patterns
- Common in real-world APIs
- Straightforward implementation (3 operations)

**Scenarios**:
1. Header Eq operation - exact match
2. Header Eq - case-insensitive header name
3. Header Exist operation - presence check
4. Header Exist - missing header not cached
5. Header In operation - value in list
6. Header In - value not in list
7. Multiple headers with same name
8. Custom vs standard headers
9. Empty header values
10. Header extractor - cache key generation

---

## Test Infrastructure

### Available Cucumber Steps

**Given**:
- `hitbox with policy` - Configure cache policy from YAML
- `request predicates` - Set request predicates from YAML
- `response predicates` - Set response predicates from YAML
- `key extractors` - Set extractors from YAML
- `mock time is enabled/disabled/reset` - Time control

**When**:
- `execute request` - Execute HURL format request
- `sleep {int}` - Advance mock time or real sleep

**Then**:
- `response status is {int}` - Check status code
- `response body jq {string}` - Validate with JQ
- `response header {string} is {string}` - Check header value
- `response headers contain/have no {string} header` - Header existence
- `cache has {int} records` - Verify cache size
- `cache key {string} exists` - Check cache key

### Configuration Format

**YAML Examples**:

```yaml
# Policy
!Enabled
ttl: 120
stale: 60

# Request Predicates
- Method: GET
- Path: /api/users/{id}
- Header:
    x-api-key: "secret"
- Query:
    operation: Eq
    page: "1"
- Body:
    parser: Jq
    jq: ".operationName"
    operation: Exist

# Response Predicates
- Status: 200
- Body:
    parser: Jq
    jq: ".error"
    operation: Exist

# Extractors
- Method:
- Path: "/api/users/{id}"
- Query: "page"
- Header: "x-tenant-id"
```

---

## Success Metrics

### Coverage Goals

- **Critical Features**: 100% BDD coverage (54/54 scenarios)
- **High Priority Features**: 100% BDD coverage (25/25 scenarios)
- **Medium Priority Features**: 80%+ coverage (65/82 scenarios)
- **Low Priority Features**: 50%+ coverage (13/26 scenarios)

### Quality Targets

- All BDD scenarios pass consistently
- No flaky tests (especially with mock time)
- Test execution time < 5 minutes for full suite
- Clear failure messages for debugging
- Code coverage > 80% for HTTP layer

---

## Dependencies & Considerations

### External Dependencies
- Cucumber Rust framework
- HURL for HTTP request format
- JQ (jaq) for JSON queries
- Mock time support in hitbox-core

### Test Isolation
- Use `@serial` tag for mock time tests
- Each scenario resets cache backend
- Independent test data per scenario
- No shared state between feature files

### CI/CD Integration
```bash
# Run all integration tests
cargo test --test integration

# Run with serial tests
cargo test --test integration -- --tags @serial --concurrency 1

# Run specific feature
cargo test --test integration -- "Header Predicates"
```

---

## Maintenance

### Updating This Plan

As implementation progresses:
1. Mark completed phases with ✅
2. Update scenario counts if scope changes
3. Add new phases if requirements emerge
4. Adjust timeline based on velocity
5. Document lessons learned

### Adding New Features

When adding new functionality:
1. Update relevant phase with new scenarios
2. Adjust total scenario count
3. Update priority if critical
4. Add to appropriate timeline week
5. Create/update feature file

---

## Conclusion

This overview provides a high-level roadmap for implementing comprehensive BDD test coverage for the Hitbox caching framework.

**Key Takeaways**:
- **187 scenarios** across **9 phases** and **22 feature files**
- **Must-Have features** (62 scenarios) deliverable in **6 weeks**
- **Full coverage** achievable in **9-13 weeks**
- Focus on **critical gaps** first (headers, body predicates, stale cache)
- Validate with **real-world scenarios** (REST API, multi-tenant)

**Next Steps**:
1. Review and approve this plan
2. Start with `header-predicates.feature` (10 scenarios)
3. Follow phased approach per priority
4. Track progress and adjust as needed

---

**For detailed implementation guidance, see**: `BDD_TEST_PLAN.md`

**Document Version**: 1.0
**Last Updated**: 2025-10-14
**Maintained By**: Hitbox Test Team
