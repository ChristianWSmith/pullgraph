use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::inner::PullGraphInner;

pub(crate) struct ActivationToken {
    pub(crate) type_id: TypeId,
    pub(crate) inner: Arc<PullGraphInner>,
    #[allow(dead_code)]
    pub(crate) dep_tokens: Vec<ActivationToken>,
}

impl Drop for ActivationToken {
    fn drop(&mut self) {
        self.inner.release_activation(self.type_id);
    }
}

/// An RAII handle representing active demand for a value of type `T`.
///
/// Creating a `Demand<T>` (via [`PullGraph::want`](crate::PullGraph::want)) increments
/// demand for `T` and activates its dependency chain. Dropping it decrements demand
/// and deactivates providers that are no longer needed.
///
/// Cloning a `Demand<T>` acquires an independent activation reference.
pub struct Demand<T: 'static> {
    pub(crate) token: ActivationToken,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T: 'static> Demand<T> {
    pub(crate) fn new(token: ActivationToken) -> Self {
        Self {
            token,
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static> Clone for Demand<T> {
    fn clone(&self) -> Self {
        let new_token = self.token.inner.acquire_activation(self.token.type_id);
        Self {
            token: new_token,
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static> Demand<T> {
    /// Returns the [`TypeId`] of the type this demand is for.
    pub fn type_id(&self) -> TypeId {
        self.token.type_id
    }
}
