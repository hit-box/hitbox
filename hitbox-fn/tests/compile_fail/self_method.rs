use hitbox_derive::cached;

struct Service;

impl Service {
    #[cached]
    pub async fn method(&self, x: i64) -> i64 {
        x * 2
    }
}

fn main() {}
