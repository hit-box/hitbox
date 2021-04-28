use std::fmt::Debug;

#[derive(Debug)]
pub struct Finish<T: Debug>
{
    pub result: T
}

impl<T> Finish<T>
where
    T: Debug
{
    pub fn result(self) -> T {
        self.result
    }
}
