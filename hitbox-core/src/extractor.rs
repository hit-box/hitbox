use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;

use crate::KeyParts;

// @FIX: remove Debug bound for Extractor (needed for hitbox-test)
#[async_trait]
pub trait Extractor: Debug {
    type Subject;
    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject>;
}

#[async_trait]
impl<T> Extractor for &T
where
    T: Extractor + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.get(subject).await
    }
}

#[async_trait]
impl<T> Extractor for Box<T>
where
    T: Extractor + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.as_ref().get(subject).await
    }
}

#[async_trait]
impl<T> Extractor for Arc<T>
where
    T: Extractor + Send + Sync + ?Sized,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn get(&self, subject: T::Subject) -> KeyParts<T::Subject> {
        self.as_ref().get(subject).await
    }
}
