use std::sync::Arc;

use axum::{routing::get, Router};
use dashmap::DashMap;
use serde::Serialize;

use crate::handlers::{get_books, get_simple};

#[derive(Hash, Eq, PartialEq, Clone, Debug, Serialize)]
pub(crate) struct AuthorId(String);

impl AuthorId {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        AuthorId(id.into())
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Debug, Serialize)]
pub(crate) struct BookId(String);

impl BookId {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        BookId(id.into())
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Book {
    id: BookId,
    author: AuthorId,
    title: String,
    description: String,
}

#[derive(Debug, Serialize)]
pub struct Author {
    id: AuthorId,
    name: String,
    family: String,
}

#[derive(Debug)]
pub(crate) struct Database {
    books: DashMap<BookId, Arc<Book>>,
    authors: DashMap<AuthorId, Arc<Author>>,
    books_by_author_idx: DashMap<AuthorId, Vec<Arc<Book>>>,
}

impl Database {
    pub(crate) fn new() -> Self {
        let books = DashMap::new();
        let authors = DashMap::new();

        let sheckley_id = AuthorId::new("robert-sheckley");
        let sheckley = Author {
            id: sheckley_id.clone(),
            name: "Robert".to_owned(),
            family: "Sheckley".to_owned(),
        };
        authors.insert(sheckley_id.clone(), Arc::new(sheckley));

        let victim_prime_id = BookId::new("victim-prime");
        let victim_prime = Book {
            id: victim_prime_id.clone(),
            author: sheckley_id.clone(),
            title: "Victim Prime".to_owned(),
            description: "Victim Prime is a science fiction novel by American writer Robert Sheckley, published in 1987. It is the sequel to 1953's \"Seventh Victim\", and is followed by 1988's Hunter/Victim.".to_owned()
        };
        books.insert(victim_prime_id, Arc::new(victim_prime));

        let journey_id = BookId::new("journey-beyond-tomorrow");
        let journey = Book {
            id: journey_id.clone(),
            author: sheckley_id.clone(),
            title: "Journey Beyond Tomorrow".to_owned(),
            description: "Journey Beyond Tomorrow is a satirical 1962 science fiction novel by the American writer Robert Sheckley, originally serialized as The Journey of Joenes in The Magazine of Fantasy & Science Fiction in October and November 1962.".to_owned() 
        };
        books.insert(journey_id, Arc::new(journey));

        let books_by_author_idx = DashMap::new();
        let sheckley_books = books
            .iter()
            .filter(|book| book.value().author == sheckley_id)
            .map(|book| Arc::clone(book.value()))
            .collect();
        books_by_author_idx.insert(sheckley_id.clone(), sheckley_books);

        Database {
            books,
            authors,
            books_by_author_idx,
        }
    }

    pub(crate) async fn get_books(&self, author_id: AuthorId) -> Option<Vec<Arc<Book>>> {
        self.books_by_author_idx
            .get(&author_id)
            .map(|entry| entry.value().clone())
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
        .route("/greet/{name}", get(get_simple))
        .route("/v1/authors/{author_id}/books", get(get_books))
        .with_state(AppState::new())
}
