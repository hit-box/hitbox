use hitbox_derive::cached;

#[cached]
pub fn not_async(x: i64) -> i64 {
    x * 2
}

fn main() {}
