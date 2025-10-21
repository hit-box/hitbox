use std::sync::Arc;

use axum::{routing::get, Router};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::handlers::{get_book, get_books, post_book};

#[derive(Hash, Eq, PartialEq, Clone, Debug, Serialize, Deserialize, Ord, PartialOrd)]
pub(crate) struct AuthorId(String);

impl AuthorId {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        AuthorId(id.into())
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize, Ord, PartialOrd)]
pub(crate) struct BookId(String);

impl BookId {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        BookId(id.into())
    }
}

#[derive(Debug, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct Book {
    id: BookId,
    author: AuthorId,
    title: String,
    description: String,
}

impl Book {
    pub(crate) fn new(id: BookId, author: AuthorId, title: String, description: String) -> Self {
        Book {
            id,
            author,
            title,
            description,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    id: AuthorId,
    name: String,
    family: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Database {
    books: DashMap<BookId, Arc<Book>>,
    authors: DashMap<AuthorId, Arc<Author>>,
    #[serde(skip)]
    books_by_author_idx: DashMap<AuthorId, Vec<Arc<Book>>>,
}

impl Database {
    pub(crate) fn new() -> Self {
        let data_content = std::fs::read_to_string("data.yaml")
            .or_else(|_| std::fs::read_to_string("hitbox-test/data.yaml"))
            .expect("Failed to read data.yaml from current directory or hitbox-test/");
        let database: Database =
            serde_yaml::from_str(&data_content).expect("Failed to parse data.yaml");

        // Build books_by_author_idx from books data
        for book_entry in database.books.iter() {
            let book = Arc::clone(book_entry.value());
            database
                .books_by_author_idx
                .entry(book.author.clone())
                .or_default()
                .push(book);
        }

        // Sort books for each author
        for mut entry in database.books_by_author_idx.iter_mut() {
            entry.value_mut().sort();
        }

        database
    }

    pub(crate) async fn get_books(&self, author_id: AuthorId) -> Option<Vec<Arc<Book>>> {
        self.books_by_author_idx
            .get(&author_id)
            .map(|entry| entry.value().clone())
    }

    pub(crate) async fn get_book(&self, book_id: BookId) -> Option<Arc<Book>> {
        self.books
            .get(&book_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    pub(crate) fn create_book(&self, book: Arc<Book>) {
        // Add to books map
        self.books.insert(book.id.clone(), Arc::clone(&book));

        // Add to author index
        self.books_by_author_idx
            .entry(book.author.clone())
            .or_default()
            .push(Arc::clone(&book));

        // Re-sort books for this author
        if let Some(mut entry) = self.books_by_author_idx.get_mut(&book.author) {
            entry.value_mut().sort();
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AppState {
    database: Arc<Database>,
}

impl AppState {
    pub(crate) fn new() -> Self {
        AppState {
            database: Arc::new(Database::new()),
        }
    }

    pub(crate) fn database(&self) -> &Database {
        &self.database
    }
}

pub(crate) fn app() -> Router {
    Router::new()
        .route("/v1/authors/{author_id}/books", get(get_books))
        .route(
            "/v1/authors/{author_id}/books/{book_id}",
            get(get_book).post(post_book),
        )
        .with_state(AppState::new())
}
