//! Example demonstrating MockTime usage for testing time-dependent cache behavior.

use hitbox_test::time::MockTime;
use std::time::Duration;

fn main() {
    println!("=== MockTime Demo ===\n");

    // Create a new mock time
    let mock_time = MockTime::new();
    println!("1. Created MockTime at: {:?}", mock_time.now());
    println!("   Elapsed: {:?}\n", mock_time.elapsed());

    // Advance time by 10 seconds
    println!("2. Advancing time by 10 seconds...");
    mock_time.advance_secs(10);
    println!("   Current time: {:?}", mock_time.now());
    println!("   Elapsed: {:?}\n", mock_time.elapsed());

    // Advance time by a duration
    println!("3. Advancing time by 5 minutes...");
    mock_time.advance(Duration::from_secs(300));
    println!("   Current time: {:?}", mock_time.now());
    println!("   Elapsed: {:?}\n", mock_time.elapsed());

    // Clone shares the same state
    println!("4. Testing clone...");
    let mock_time2 = mock_time.clone();
    println!("   mock_time1 now: {:?}", mock_time.now());
    println!("   mock_time2 now: {:?}", mock_time2.now());
    println!(
        "   Both times are equal: {}\n",
        mock_time.now() == mock_time2.now()
    );

    // Advancing one affects both
    println!("5. Advancing mock_time2 by 1 hour...");
    mock_time2.advance(Duration::from_secs(3600));
    println!("   mock_time1 now: {:?}", mock_time.now());
    println!("   mock_time2 now: {:?}", mock_time2.now());
    println!("   Total elapsed: {:?}\n", mock_time.elapsed());

    // Reset time
    println!("6. Resetting time...");
    mock_time.reset();
    println!("   Current time: {:?}", mock_time.now());
    println!("   Elapsed after reset: {:?}\n", mock_time.elapsed());

    // Practical example: simulating cache TTL
    println!("=== Simulating Cache TTL ===\n");
    simulate_cache_ttl();
}

fn simulate_cache_ttl() {
    let mock_time = MockTime::new();
    let cache_ttl = Duration::from_secs(30); // 30 second TTL

    // Simulate cache write
    let cached_at = mock_time.now();
    println!("1. Item cached at: {:?}", cached_at);

    // Check if expired after 15 seconds
    mock_time.advance_secs(15);
    let current = mock_time.now();
    let elapsed = current.duration_since(cached_at).unwrap();
    let expired = elapsed > cache_ttl;
    println!("2. After 15 seconds:");
    println!("   Elapsed: {:?}", elapsed);
    println!("   Expired: {} (TTL: {:?})", expired, cache_ttl);

    // Check if expired after 35 seconds total
    mock_time.advance_secs(20); // Total: 35 seconds
    let current = mock_time.now();
    let elapsed = current.duration_since(cached_at).unwrap();
    let expired = elapsed > cache_ttl;
    println!("3. After 35 seconds:");
    println!("   Elapsed: {:?}", elapsed);
    println!("   Expired: {} (TTL: {:?})", expired, cache_ttl);
}
