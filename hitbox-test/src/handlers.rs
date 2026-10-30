use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
};
use http::{HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};

use crate::app::{AppState, AuthorId, Book, BookId};
use crate::handler_state::HandlerName;

/// Echo handler that accepts any body and returns 200 with the body echoed back.
/// Useful for testing body extractors.
#[axum::debug_handler]
pub(crate) async fn echo_body(State(state): State<AppState>, body: Bytes) -> Response {
    state
        .handler_state
        .increment_call_count(HandlerName::EchoBody);
    state.handler_state.apply_delay(HandlerName::EchoBody).await;

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/octet-stream")
        .body(axum::body::Body::from(body))
        .unwrap()
}

const DEFAULT_PER_PAGE: usize = 3;

#[derive(Deserialize, Debug)]
pub struct Pagination {
    page: Option<usize>,
    per_page: Option<usize>,
}

#[derive(Deserialize, Debug, Default)]
pub struct QueryParams {
    test_headers: Option<String>,
    streaming: Option<String>,
}

#[axum::debug_handler]
pub(crate) async fn get_book(
    State(state): State<AppState>,
    Path((_author_id, book_id)): Path<(String, String)>,
    Query(query): Query<QueryParams>,
) -> Result<Response, StatusCode> {
    state
        .handler_state
        .increment_call_count(HandlerName::GetBook);
    state.handler_state.apply_delay(HandlerName::GetBook).await;

    match book_id.as_str() {
        "invalid-book-id" => Err(StatusCode::INTERNAL_SERVER_ERROR),
        _ => {
            let book = state
                .database()
                .get_book(BookId::new(&book_id))
                .await
                .ok_or(StatusCode::NOT_FOUND)?;

            // Serialize book to JSON
            let json_bytes =
                serde_json::to_vec(&*book).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Handle streaming response if streaming=true query param is present
            if query.streaming.as_deref() == Some("true") {
                use futures::stream;
                use http::header;
                use http_body_util::StreamBody;

                // Split JSON into chunks to test pattern spanning boundaries
                // We'll split at a known position to create a boundary in the middle of "robert-sheckley"
                let chunk_size = 50; // Small chunks to force boundaries
                let chunks: Vec<Result<_, std::io::Error>> = json_bytes
                    .chunks(chunk_size)
                    .map(|chunk| {
                        let data = Bytes::copy_from_slice(chunk);
                        Ok(http_body::Frame::data(data))
                    })
                    .collect();

                let stream = stream::iter(chunks);
                let stream_body = StreamBody::new(stream);

                let mut response = Response::new(axum::body::Body::new(stream_body));
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                return Ok(response);
            }

            // Add custom headers for testing if test_headers=true query param is present
            if query.test_headers.as_deref() == Some("true") {
                let json_response = Json(book).into_response();
                let (mut parts, body) = json_response.into_parts();

                parts
                    .headers
                    .insert("server", HeaderValue::from_static("hitbox-test"));
                // Canonical response-tag source for tag-invalidation tests.
                parts
                    .headers
                    .insert("etag", HeaderValue::from_static("book-v1"));
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
    state
        .handler_state
        .increment_call_count(HandlerName::GetBooks);
    state.handler_state.apply_delay(HandlerName::GetBooks).await;

    let mut books = state
        .database()
        .get_books(AuthorId::new(author_id))
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Sort books by ID for deterministic ordering
    books.sort();

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
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

#[axum::debug_handler]
pub(crate) async fn post_book(
    State(state): State<AppState>,
    Path((author_id, book_id)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<Arc<Book>>, StatusCode> {
    state
        .handler_state
        .increment_call_count(HandlerName::PostBook);
    state.handler_state.apply_delay(HandlerName::PostBook).await;

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
        request
            .description
            .unwrap_or_else(|| "No description".to_string()),
    ));

    // Store in database
    state.database().create_book(book.clone());

    // Return the created book
    Ok(Json(book))
}

#[axum::debug_handler]
pub(crate) async fn get_book_cover(
    State(state): State<AppState>,
    Path(book_id): Path<String>,
    Query(query): Query<QueryParams>,
) -> Result<Response, StatusCode> {
    state
        .handler_state
        .increment_call_count(HandlerName::GetBookCover);
    state
        .handler_state
        .apply_delay(HandlerName::GetBookCover)
        .await;

    // Try to load cover image from covers directory
    let cover_path = format!("covers/{}.png", book_id);
    let cover_path_fallback = format!("hitbox-test/covers/{}.png", book_id);

    let cover_data = std::fs::read(&cover_path)
        .or_else(|_| std::fs::read(&cover_path_fallback))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Handle streaming response if streaming=true query param is present
    if query.streaming.as_deref() == Some("true") {
        use futures::stream;
        use http::header;
        use http_body_util::StreamBody;

        // Split binary data into chunks to test pattern spanning boundaries
        let chunk_size = 20; // Small chunks to force boundaries in PNG header
        let chunks: Vec<Result<_, std::io::Error>> = cover_data
            .chunks(chunk_size)
            .map(|chunk| {
                let data = Bytes::copy_from_slice(chunk);
                Ok(http_body::Frame::data(data))
            })
            .collect();

        let stream = stream::iter(chunks);
        let stream_body = StreamBody::new(stream);

        let mut response = Response::new(axum::body::Body::new(stream_body));
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
        return Ok(response);
    }

    // Regular response
    let mut response = Response::new(axum::body::Body::from(cover_data));
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static("image/png"),
    );
    Ok(response)
}
