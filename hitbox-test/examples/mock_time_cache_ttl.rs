//! Example demonstrating how to use MockTime with CacheValue for testing TTL.
//!
//! This example shows the conceptual usage. In real tests, you would need to
//! integrate this with your specific caching implementation.

use chrono::Utc;
use hitbox_core::TimeProvider;
use hitbox_test::time::{MockTime, MockTimeProvider};
use std::time::Duration;

fn main() {
    println!("=== MockTime with CacheValue TTL Testing ===\n");

    // Create mock time
    let mock_time = MockTime::new();
    let provider = MockTimeProvider::new(mock_time.clone());

    // Simulate creating a cache value with 30 second TTL
    let now = provider.now();
    let ttl = Duration::from_secs(30);
    let expire_time = now + ttl;

    println!("1. Created cache value");
    println!("   Current time: {}", now);
    println!("   Expire time:  {}", expire_time);
    println!("   TTL: {:?}\n", ttl);

    // Check if expired (should be false)
    let current = provider.now();
    let is_expired = current >= expire_time;
    println!("2. Checking immediately:");
    println!("   Current time: {}", current);
    println!("   Is expired: {}\n", is_expired);

    // Advance time by 15 seconds
    println!("3. Advancing time by 15 seconds...");
    mock_time.advance_secs(15);
    let current = provider.now();
    let is_expired = current >= expire_time;
    println!("   Current time: {}", current);
    println!("   Is expired: {}\n", is_expired);

    // Advance time by 20 more seconds (total 35 seconds)
    println!("4. Advancing time by 20 more seconds (total 35 seconds)...");
    mock_time.advance_secs(20);
    let current = provider.now();
    let is_expired = current >= expire_time;
    println!("   Current time: {}", current);
    println!("   Is expired: {}\n", is_expired);

    // Demonstrate stale cache
    println!("=== Stale Cache Testing ===\n");

    let mock_time2 = MockTime::new();
    let provider2 = MockTimeProvider::new(mock_time2.clone());

    let now = provider2.now();
    let stale_time = now + Duration::from_secs(10);
    let expire_time = now + Duration::from_secs(30);

    println!("5. Created cache value with stale mechanic");
    println!("   Current time: {}", now);
    println!("   Stale time:   {}", stale_time);
    println!("   Expire time:  {}", expire_time);

    // Function to check cache state
    let check_state = |provider: &MockTimeProvider,
                       stale: chrono::DateTime<Utc>,
                       expire: chrono::DateTime<Utc>| {
        let current = provider.now();
        if current >= expire {
            "Expired"
        } else if current >= stale {
            "Stale"
        } else {
            "Actual"
        }
    };

    let state = check_state(&provider2, stale_time, expire_time);
    println!("   State: {}\n", state);

    // After 5 seconds - still actual
    mock_time2.advance_secs(5);
    let state = check_state(&provider2, stale_time, expire_time);
    println!("6. After 5 seconds:");
    println!("   State: {}\n", state);

    // After 15 seconds total - stale
    mock_time2.advance_secs(10);
    let state = check_state(&provider2, stale_time, expire_time);
    println!("7. After 15 seconds total:");
    println!("   State: {}\n", state);

    // After 35 seconds total - expired
    mock_time2.advance_secs(20);
    let state = check_state(&provider2, stale_time, expire_time);
    println!("8. After 35 seconds total:");
    println!("   State: {}\n", state);

    println!("=== Benefits ===");
    println!("✓ No actual waiting - tests run instantly");
    println!("✓ Precise control over time");
    println!("✓ Test edge cases easily");
    println!("✓ Deterministic test behavior");
}
