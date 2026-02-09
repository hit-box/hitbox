use hitbox_derive::cached;

#[cached(skip(nonexistent))]
pub async fn example(value: i64) -> i64 {
    value * 2
}

fn main() {}
