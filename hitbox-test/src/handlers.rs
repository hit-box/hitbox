use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use http::StatusCode;
use serde::Deserialize;

use crate::app::{AppState, AuthorId, Book, BookId};

const DEFAULT_PER_PAGE: usize = 3;

#[derive(Deserialize, Debug)]
pub struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}

#[axum::debug_handler]
pub(crate) async fn get_book(
    State(state): State<AppState>,
    Path((_author_id, book_id)): Path<(String, String)>,
) -> Result<Json<Arc<Book>>, StatusCode> {
    match book_id.as_str() {
        "invalid-book-id" => Err(StatusCode::INTERNAL_SERVER_ERROR),
        _ => {
            let book = state
                .database()
                .get_book(BookId::new(book_id))
                .await
                .ok_or(StatusCode::NOT_FOUND)?;
            Ok(Json(book))
        }
    }
}

#[axum::debug_handler]
pub(crate) async fn get_books(
    State(state): State<AppState>,
    Path(author_id): Path<String>,
    pagination: Query<Pagination>,
) -> Result<Json<Vec<Arc<Book>>>, StatusCode> {
    let books = state
        .database()
        .get_books(AuthorId::new(author_id))
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(DEFAULT_PER_PAGE);
    let start = (page - 1) * per_page;

    let paginated_books = books
        .into_iter()
        .skip(start)
        .take(per_page)
        .collect::<Vec<_>>();

    Ok(Json(paginated_books))
}
