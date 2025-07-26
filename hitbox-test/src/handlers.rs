use axum::extract::{Path, Query};
use http::StatusCode;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Pagination {
    page: usize,
    per_page: usize,
}

pub async fn get_books(
    Path(author_id): Path<String>,
    pagination: Query<Pagination>,
) -> Result<String, StatusCode> {
    dbg!(pagination);
    Ok(format!("Hello, {author_id}"))
}

pub async fn get_simple(Path(name): Path<String>) -> Result<String, StatusCode> {
    Ok(format!("Hello, {name}"))
}
