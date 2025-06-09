use axum::extract::Path;
use http::StatusCode;

pub async fn handler_result(
    Path(name): Path<String>,
    _request: axum::extract::Request,
) -> Result<String, StatusCode> {
    dbg!("axum::handler_result");
    Ok(format!("Hello, {name}"))
}
