pub trait Predicate<S> {
    fn check(&self, subject: &S) -> bool;
}

pub enum Operation {
    Eq,
    In,
    // TODO: extend predicate operations
}
