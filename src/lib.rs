pub mod actor;
pub mod backend;
pub mod cache;
pub mod error;

pub use error::CacheError;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
