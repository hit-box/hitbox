# Implementation Task: Efficient Body Size Limit with BufferedBody Wrapper

## Context

Working on the `tower` branch of hitbox. The current implementation uses a `FromBytes` trait to reconstruct bodies after consuming them in predicates. We want to replace this with a more efficient wrapper-based approach.

**Current architecture:**
- `hitbox-http/src/body.rs` has `FromBytes` trait
- `hitbox-http/src/predicates/request/body.rs` shows the pattern: consume body, analyze, reconstruct
- `hitbox-tower/src/service.rs` defines `CacheService` that currently requires `S: Service<Request<ReqBody>>`

## Goal

Implement an efficient body size limit predicate that:
1. **Doesn't consume large bodies** - stops reading when size limit exceeded
2. **Preserves the full body** for upstream via a wrapper
3. **Uses zero-cost wrapper type** instead of boxing or reconstruction

## Design

### 1. Create `BufferedBody<B>` enum

Location: `hitbox-http/src/buffered_body.rs` (new file)

```rust
/// A body wrapper that represents different consumption states.
/// This allows predicates to partially consume bodies without losing data.
pub enum BufferedBody<B> {
    /// Body was fully read and buffered (within size limits)
    Complete(Bytes),

    /// Body was partially read - has buffered prefix + remaining unconsumed stream
    Partial {
        prefix: Option<Bytes>,  // Option to yield once, then None
        remaining: B
    },

    /// Body was passed through without reading (untouched)
    Passthrough(B),
}
```

**Requirements:**
- Implement `http_body::Body` trait for `BufferedBody<B> where B: Body`
- Use `pin_project` crate for safe pinning
- `poll_data` should:
  - For `Complete`: yield bytes once, then None
  - For `Partial`: yield prefix first, then delegate to remaining body
  - For `Passthrough`: delegate directly to inner body
- Implement `poll_trailers` by delegating to inner (if present)
- Implement `size_hint` correctly for all variants
- Implement `is_end_stream` correctly

### 2. Create Body Size Limit Predicate

Location: `hitbox-http/src/predicates/request/body_size_limit.rs` (new file)

```rust
pub struct BodySizeLimit<P> {
    inner: P,
    max_size: usize,
}

pub trait BodySizeLimitPredicate: Sized {
    fn body_size_limit(self, max_size: usize) -> BodySizeLimit<Self>;
}
```

**Requirements:**
- Implement `Predicate` trait for `BodySizeLimit<P>`
- In `check()` method:
  - First check `size_hint().upper()` - if available and exceeds limit, immediately return `NonCacheable` with `Passthrough` variant
  - Otherwise, consume body chunks incrementally up to `max_size`
  - If size exceeds limit during reading: return `NonCacheable` with `Partial` variant
  - If body completes within limit: return `Cacheable` with `Complete` variant
- Handle errors gracefully (body read errors should result in `NonCacheable`)
- Add `tracing::debug!` logs for debugging

### 3. Update Service Signature

Location: `hitbox-tower/src/service.rs`

**Change the service to accept `BufferedBody<ReqBody>`:**

```rust
impl<S, B, C, ReqBody, ResBody> Service<Request<ReqBody>> for CacheService<S, B, C>
where
    S: Service<Request<BufferedBody<ReqBody>>, Response = Response<ResBody>> + Clone + Send + 'static,
    //             ^^^^^^^^^^^^^^^^^^^^ Wrapped body type
```

Update all related bounds and the `Transformer` to handle `BufferedBody<ReqBody>`.

### 4. Update or Remove FromBytes

**Option A:** Keep `FromBytes` for the `Complete` case but add:
```rust
impl<B: HttpBody> FromBytes for BufferedBody<B> {
    fn from_bytes(bytes: Bytes) -> Self {
        BufferedBody::Complete(bytes)
    }
}
```

**Option B:** Remove `FromBytes` entirely if it's no longer needed after refactoring.

### 5. Export and Integrate

- Export `BufferedBody` from `hitbox-http/src/lib.rs`
- Export `BodySizeLimit` and `BodySizeLimitPredicate` from `hitbox-http/src/predicates/request/mod.rs`
- Update `hitbox-http/src/predicates/mod.rs` exports
- Ensure all existing body predicates work with `BufferedBody`

## Implementation Details

### Pin projection example for `BufferedBody`:

```rust
use pin_project::pin_project;

#[pin_project(project = BufferedBodyProj)]
pub enum BufferedBody<B> {
    Complete(Bytes),  // No pin needed - Bytes is Unpin
    Partial {
        prefix: Option<Bytes>,
        #[pin]
        remaining: B
    },
    Passthrough(#[pin] B),
}
```

### Incremental consumption in predicate:

```rust
let mut total_size = 0;
let mut buffer = BytesMut::new();
let mut body_pin = std::pin::pin!(body);

loop {
    match body_pin.as_mut().frame().await {
        Some(Ok(frame)) => {
            if let Some(data) = frame.into_data().ok() {
                total_size += data.len();

                if total_size > self.max_size {
                    // Exceeded! Create Partial variant
                    let buffered = buffer.freeze();
                    return PredicateResult::NonCacheable(
                        CacheableHttpRequest::from_request(
                            Request::from_parts(
                                parts,
                                BufferedBody::Partial {
                                    prefix: Some(buffered),
                                    remaining: body_pin.into_inner(),
                                }
                            )
                        )
                    );
                }

                buffer.extend_from_slice(&data);
            }
        }
        Some(Err(_)) => { /* handle error */ }
        None => {
            // Body complete within limit
            return PredicateResult::Cacheable(/* ... */);
        }
    }
}
```

## Testing

Create tests in `hitbox-http/tests/buffered_body.rs`:
1. Test `BufferedBody::Complete` yields all bytes then None
2. Test `BufferedBody::Partial` yields prefix then remaining chunks
3. Test `BufferedBody::Passthrough` forwards all chunks
4. Test `BodySizeLimit` with body under limit (should be cacheable)
5. Test `BodySizeLimit` with body over limit (should be non-cacheable)
6. Test that large bodies don't get fully consumed
7. Test reconstruction preserves all body data

## Dependencies

Ensure `Cargo.toml` has:
```toml
http-body = "1"
http-body-util = "0.1"
pin-project = "1"
bytes = "1"
```

## Success Criteria

- [ ] `BufferedBody` implements `http_body::Body` correctly
- [ ] `BodySizeLimit` predicate stops reading at limit
- [ ] Large bodies passed to upstream without full consumption
- [ ] All tests pass
- [ ] Service compiles with new signature
- [ ] Existing predicates still work
