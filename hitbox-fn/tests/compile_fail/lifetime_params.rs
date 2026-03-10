use hitbox_derive::cached;

#[cached]
pub async fn example<'a>(value: &'a str) -> String {
    value.to_string()
}

fn main() {}
