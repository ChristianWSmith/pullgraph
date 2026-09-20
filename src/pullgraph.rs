use std::any::TypeId;
use std::sync::Arc;

use crate::context::Context;
use crate::demand::Demand;
use crate::error::PullGraphError;
use crate::inner::PullGraphInner;
use crate::provider::{ContextFn, DepHandle, Provider, ProviderHandle};

/// The central registry for demand-driven computation.
///
/// Register providers with [`provide`](PullGraph::provide),
/// [`provide_with_deps`](PullGraph::provide_with_deps), or
/// [`derive`](PullGraph::derive). Create demand with
/// [`want`](PullGraph::want). Production is triggered by calling
/// [`produce`](Context::produce) on a [`Context`].
pub struct PullGraph {
    inner: Arc<PullGraphInner>,
}

impl Default for PullGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl PullGraph {
    /// Create a new, empty demand graph.
    pub fn new() -> Self {
        #[allow(clippy::arc_with_non_send_sync)]
        Self {
            inner: Arc::new(PullGraphInner::new()),
        }
    }

    /// Register a provider for type `T` with no dependencies.
    ///
    /// Returns a [`ProviderHandle<T>`](ProviderHandle) that can be used to
    /// declare dependencies in subsequent registrations.
    pub fn provide<T: 'static>(
        &self,
        provider: impl Provider<T> + 'static,
    ) -> Result<ProviderHandle<T>, PullGraphError> {
        self.inner.register_provider(provider, Vec::new())?;
        Ok(ProviderHandle::new())
    }

    /// Register a provider for type `T` with explicit dependencies.
    ///
    /// The provider closure receives a [`Context`] reference so it can read
    /// previously produced dependency values. Dependencies are validated for
    /// cycles at registration time.
    pub fn provide_with_deps<T: 'static>(
        &self,
        deps: &[&dyn DepHandle],
        provider: impl FnMut(&mut Context) -> T + 'static,
    ) -> Result<ProviderHandle<T>, PullGraphError> {
        let dep_type_ids: Vec<TypeId> = deps.iter().map(|d| d.type_id()).collect();

        {
            let state = self.inner.state.lock().unwrap();
            for &dep_id in &dep_type_ids {
                let type_id = TypeId::of::<T>();
                if crate::graph::has_cycle(&state.dependents, type_id, dep_id) {
                    return Err(PullGraphError::DependencyCycle(vec![type_id, dep_id]));
                }
            }
        }

        self.inner
            .register_provider(ContextFn(provider), dep_type_ids)?;
        Ok(ProviderHandle::new())
    }

    /// Register a derived provider: a value produced by transforming dependencies.
    ///
    /// Convenience wrapper around [`provide_with_deps`](PullGraph::provide_with_deps).
    /// The transform closure receives a [`Context`] and returns the derived value.
    pub fn derive<T, F>(
        &self,
        deps: &[&dyn DepHandle],
        transform: F,
    ) -> Result<ProviderHandle<T>, PullGraphError>
    where
        T: 'static,
        F: FnMut(&mut Context) -> T + 'static,
    {
        let dep_type_ids: Vec<TypeId> = deps.iter().map(|d| d.type_id()).collect();

        {
            let state = self.inner.state.lock().unwrap();
            for &dep_id in &dep_type_ids {
                let type_id = TypeId::of::<T>();
                if crate::graph::has_cycle(&state.dependents, type_id, dep_id) {
                    return Err(PullGraphError::DependencyCycle(vec![type_id, dep_id]));
                }
            }
        }

        self.inner
            .register_provider(ContextFn(transform), dep_type_ids)?;
        Ok(ProviderHandle::new())
    }

    /// Create demand for a value of type `T`.
    ///
    /// Returns a [`Demand<T>`](Demand) handle. The demand is active for as long
    /// as the handle exists. Dropping it releases demand and may deactivate
    /// providers. Returns [`PullGraphError::NoProvider`] if no provider is
    /// registered for `T`.
    pub fn want<T: 'static>(&self) -> Result<Demand<T>, PullGraphError> {
        let type_id = TypeId::of::<T>();
        {
            let state = self.inner.state.lock().unwrap();
            if !state.providers.contains_key(&type_id) {
                return Err(PullGraphError::NoProvider(type_id));
            }
        }

        let token = self.inner.acquire_activation(type_id);
        Ok(Demand::new(token))
    }

    /// Returns `true` if at least one active demand exists for type `T`.
    pub fn is_demanded<T: 'static>(&self) -> bool {
        self.inner.is_demanded(TypeId::of::<T>())
    }

    /// Returns the number of active demand references for type `T`.
    pub fn demand_count<T: 'static>(&self) -> usize {
        self.inner.demand_count(TypeId::of::<T>())
    }

    /// Create a [`Context`] for one execution step.
    ///
    /// Call [`Context::produce`] to produce demanded values within this context.
    pub fn context(&self) -> Context {
        Context::new(Arc::clone(&self.inner))
    }

    #[cfg(test)]
    pub(crate) fn inner(&self) -> &Arc<PullGraphInner> {
        &self.inner
    }
}

impl Clone for PullGraph {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_creates_pullgraph() {
        let pullgraph = PullGraph::default();
        assert!(!pullgraph.is_demanded::<i32>());
    }

    #[test]
    fn clone_shares_state() {
        let pullgraph1 = PullGraph::new();
        let _h = pullgraph1.provide(|| 42i32).unwrap();
        let pullgraph2 = pullgraph1.clone();
        let _d = pullgraph2.want::<i32>().unwrap();
        assert!(pullgraph1.is_demanded::<i32>());
    }

    #[test]
    fn provide_with_deps_self_cycle_detected() {
        let pullgraph = PullGraph::new();
        let _h = ProviderHandle::<f64>::new();
        let deps: &[&dyn DepHandle] = &[&_h];
        let result = pullgraph.provide_with_deps(deps, |_: &mut Context| 1.0f64);
        assert!(matches!(result, Err(PullGraphError::DependencyCycle(_))));
    }

    #[test]
    fn provide_with_deps_transitive_cycle_detected() {
        let pullgraph = PullGraph::new();
        let h1 = pullgraph.provide(|| 1i32).unwrap();
        let deps1: &[&dyn DepHandle] = &[&h1];
        let h2: ProviderHandle<i64> = pullgraph
            .derive(deps1, |ctx| -> i64 {
                ctx.get::<i32>().map(|&v| v as i64).unwrap_or(0)
            })
            .unwrap();

        {
            let mut state = pullgraph.inner.state.lock().unwrap();
            let f64_id = TypeId::of::<f64>();
            let i64_id = TypeId::of::<i64>();
            state.dependents.insert(f64_id, vec![i64_id]);
        }
        let deps: &[&dyn DepHandle] = &[&h2];
        let result = pullgraph.provide_with_deps(deps, |_: &mut Context| 1.0f64);
        assert!(matches!(result, Err(PullGraphError::DependencyCycle(_))));
    }

    #[test]
    fn derive_self_cycle_detected() {
        let pullgraph = PullGraph::new();
        let _h = ProviderHandle::<f64>::new();
        let deps: &[&dyn DepHandle] = &[&_h];
        let result = pullgraph.derive(deps, |_: &mut Context| 1.0f64);
        assert!(matches!(result, Err(PullGraphError::DependencyCycle(_))));
    }

    #[test]
    fn derive_transitive_cycle_detected() {
        let pullgraph = PullGraph::new();
        let h1 = pullgraph.provide(|| 1i32).unwrap();
        let deps1: &[&dyn DepHandle] = &[&h1];
        let h2: ProviderHandle<i64> = pullgraph
            .derive(deps1, |ctx| -> i64 {
                ctx.get::<i32>().map(|&v| v as i64).unwrap_or(0)
            })
            .unwrap();

        {
            let mut state = pullgraph.inner.state.lock().unwrap();
            let f64_id = TypeId::of::<f64>();
            let i64_id = TypeId::of::<i64>();
            state.dependents.insert(f64_id, vec![i64_id]);
        }
        let deps: &[&dyn DepHandle] = &[&h2];
        let result = pullgraph.derive(deps, |_: &mut Context| 1.0f64);
        assert!(matches!(result, Err(PullGraphError::DependencyCycle(_))));
    }
}
