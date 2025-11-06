use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Json,
};
use http::{HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};

use crate::app::{AppState, AuthorId, Book, BookId};

const DEFAULT_PER_PAGE: usize = 3;

#[derive(Deserialize, Debug)]
pub struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}

#[derive(Deserialize, Debug, Default)]
pub struct TestHeadersQuery {
    test_headers: Option<String>,
}

#[axum::debug_handler]
pub(crate) async fn get_book(
    State(state): State<AppState>,
    Path((_author_id, book_id)): Path<(String, String)>,
    Query(query): Query<TestHeadersQuery>,
) -> Result<Response, StatusCode> {
    match book_id.as_str() {
        "invalid-book-id" => Err(StatusCode::INTERNAL_SERVER_ERROR),
        _ => {
            let book = state
                .database()
                .get_book(BookId::new(&book_id))
                .await
                .ok_or(StatusCode::NOT_FOUND)?;

            // Add custom headers for testing if test_headers=true query param is present
            if query.test_headers.as_deref() == Some("true") {
                let json_response = Json(book).into_response();
                let (mut parts, body) = json_response.into_parts();

                parts
                    .headers
                    .insert("server", HeaderValue::from_static("hitbox-test"));
                parts
                    .headers
                    .insert("x-empty", HeaderValue::from_static(""));
                parts
                    .headers
                    .insert("x-custom", HeaderValue::from_static("  value  "));
                parts
                    .headers
                    .insert("set-cookie", HeaderValue::from_static("session=abc123"));
                parts
                    .headers
                    .append("set-cookie", HeaderValue::from_static("token=xyz789"));

                Ok(Response::from_parts(parts, body))
            } else {
                Ok(Json(book).into_response())
            }
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

#[derive(Deserialize, Serialize, Debug)]
pub(crate) struct CreateBookRequest {
    title: String,
    description: String,
}

#[axum::debug_handler]
pub(crate) async fn post_book(
    State(state): State<AppState>,
    Path((author_id, book_id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<Arc<Book>>, StatusCode> {
    // Check if book already exists
    if state
        .database()
        .get_book(BookId::new(&book_id))
        .await
        .is_some()
    {
        return Err(StatusCode::CONFLICT);
    }

    // Parse the body as CreateBookRequest
    let request: CreateBookRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Create the book
    let book = Arc::new(Book::new(
        BookId::new(book_id),
        AuthorId::new(author_id),
        request.title,
        request.description,
    ));

    // Store in database
    state.database().create_book(book.clone());

    // Return the created book
    Ok(Json(book))
}
