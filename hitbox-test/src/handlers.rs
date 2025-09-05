use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use http::StatusCode;
use serde::Deserialize;

use crate::app::{AppState, AuthorId, Book, BookId};

#[derive(Deserialize, Debug)]
pub struct Pagination {
    #[allow(dead_code)]
    page: usize,
    #[allow(dead_code)]
    per_page: usize,
}

#[axum::debug_handler]
pub(crate) async fn get_books(
    State(state): State<AppState>,
    Path(author_id): Path<String>,
    _pagination: Query<Pagination>,
) -> Result<Json<Vec<Arc<Book>>>, StatusCode> {
    let books = Json(
        state
            .database()
            .get_books(AuthorId::new(author_id))
            .await
            .ok_or(StatusCode::NOT_FOUND)?,
    );
    Ok(books)
}

pub async fn get_simple(Path(name): Path<String>) -> Result<String, StatusCode> {
    Ok(format!("Hello, {name}"))
}

#[axum::debug_handler]
pub(crate) async fn get_book(
    State(state): State<AppState>,
    Path((_author_id, book_id)): Path<(String, String)>,
) -> Result<Json<Arc<Book>>, StatusCode> {
    let book = Json(
        state
            .database()
            .get_book(BookId::new(book_id))
            .await
            .ok_or(StatusCode::NOT_FOUND)?,
    );
    Ok(book)
}
