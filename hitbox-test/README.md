# hitbox-test

Testing utilities and integration tests for the Hitbox caching framework.

## Features

### MockTime

The `MockTime` type allows you to control time in tests without actually sleeping. This is particularly useful for testing cache expiration, TTL, and stale cache mechanics.

**Key Features:**
- Control time flow in tests
- Works with `CacheValue` for testing TTL/stale mechanics
- No actual waiting - tests run instantly
- Thread-safe and cloneable

#### Basic Usage

```rust
use hitbox_test::time::MockTime;
use std::time::Duration;

let mock_time = MockTime::new();
let now = mock_time.now();

// Advance time by 10 seconds without actually sleeping
mock_time.advance(Duration::from_secs(10));

let later = mock_time.now();
assert_eq!(later.duration_since(now).unwrap(), Duration::from_secs(10));
```

#### Cucumber Integration

MockTime integrates with Cucumber tests through the `HitboxWorld` state:

```gherkin
@serial
Feature: Cache TTL Testing
  Background:
    Given mock time is enabled
    And hitbox with policy
    """
    Enabled:
      ttl: 10
    """

  Scenario: Cache expires after TTL
    When execute request
    """
    GET http://localhost/api/data
    """
    Then response headers contain a header "X-Cache-Status" with value "MISS"

    # Advance time instantly without waiting
    When sleep 15

    When execute request
    """
    GET http://localhost/api/data
    """
    Then response headers contain a header "X-Cache-Status" with value "MISS"
```

**Important:** Features that use mock time must be tagged with `@serial` to ensure scenarios run sequentially. This is required because:
- Mock time uses global state shared across threads
- Async code can migrate between threads, making thread-local storage unreliable
- Time-dependent tests should run sequentially for deterministic behavior

#### Available Given Steps

- `Given mock time is enabled` - Enables mock time for the scenario
- `Given mock time is disabled` - Disables mock time (uses real sleep)
- `Given mock time is reset` - Resets mock time to its base time

#### When Steps

- `When sleep {int}` - If mock time is enabled, advances mock time by the specified seconds instantly. Otherwise, performs a real tokio sleep.

#### API

The `MockTime` type provides these methods:

- `new()` - Create a new MockTime starting at the current system time
- `with_time(time)` - Create a MockTime starting at a specific time
- `now()` - Get the current mocked time
- `advance(duration)` - Advance time by a duration
- `advance_secs(secs)` - Advance time by a number of seconds
- `elapsed()` - Get the total duration advanced
- `reset()` - Reset to the original base time
- `set_time(time)` - Set to a specific point in time

#### Thread Safety

`MockTime` is thread-safe and can be cloned. All clones share the same time state:

```rust
let mock_time1 = MockTime::new();
let mock_time2 = mock_time1.clone();

mock_time1.advance_secs(10);

// Both instances see the same time
assert_eq!(mock_time1.now(), mock_time2.now());
```

### Integration with CacheValue

`MockTime` integrates with `hitbox_core::CacheValue` to enable testing of cache TTL and stale mechanics. In test builds, `CacheValue` uses a mockable time provider.

#### Using MockTimeProvider with CacheValue

```rust
use hitbox_test::time::{MockTime, MockTimeProvider};
use hitbox_core::{CacheValue, TimeProvider};
use chrono::Utc;
use std::time::Duration;

// Create mock time
let mock_time = MockTime::new();
let provider = MockTimeProvider::new(mock_time.clone());

// Create a cache value with 30 second TTL
let now = provider.now();
let expire_time = now + Duration::from_secs(30);
let cache_value = CacheValue::new(data, Some(expire_time), None);

// Test that cache is not expired initially
// (in real tests, you'd use cache_state())

// Advance time past TTL
mock_time.advance_secs(35);

// Test that cache is now expired
// (in real tests, the cache would return CacheState::Expired)
```

#### Example: Testing Cache Expiration

```rust
use hitbox_test::time::{MockTime, MockTimeProvider};
use chrono::Utc;
use std::time::Duration;

let mock_time = MockTime::new();
let provider = MockTimeProvider::new(mock_time.clone());

// Create cache entry with 10 second stale time and 30 second TTL
let now = provider.now();
let stale_time = now + Duration::from_secs(10);
let expire_time = now + Duration::from_secs(30);

// Check state progression
println!("State at T+0:  Actual");
mock_time.advance_secs(5);
println!("State at T+5:  Actual");
mock_time.advance_secs(10); // Total: 15 seconds
println!("State at T+15: Stale");
mock_time.advance_secs(20); // Total: 35 seconds
println!("State at T+35: Expired");
```

See `examples/mock_time_cache_ttl.rs` for a complete working example.

### Important Notes

**Cucumber Tests:** Features that use mock time must be tagged with `@serial` to ensure sequential execution:
```gherkin
@serial
Feature: Cache TTL with Mock Time
  Background:
    Given mock time is enabled
```

The `@serial` tag is required because:
- Mock time uses global state
- Async Rust code can migrate between threads during `.await` points
- Sequential execution ensures deterministic time behavior

**Unit Tests:** Library tests that use `setup_mock_time_for_testing()` should either:
1. Run with `--test-threads=1` to avoid interference
2. Call `clear_mock_time_provider()` at the end of each test
3. Use unique time baselines for each test

Example running tests with isolation:
```bash
cargo test -- --test-threads=1
```

## Running Tests

Run the library tests:

```bash
cargo test --lib
```

Run integration tests:

```bash
cargo test --test integration
```

## License

MIT
