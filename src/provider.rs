use std::any::{Any, TypeId};
use std::marker::PhantomData;

use crate::context::Context;

/// A component that produces values of type `T` on demand.
///
/// Implement this trait to define how a value is produced. The `activate` and
/// `deactivate` methods are optional lifecycle hooks called when demand transitions
/// from zero to one and one to zero, respectively.
///
/// Zero-argument closures automatically implement `Provider<T>`:
///
/// ```ignore
/// pullgraph.provide(|| 42i32).unwrap();
/// ```
pub trait Provider<T: 'static> {
    /// Called when demand for `T` transitions from zero to one.
    fn activate(&mut self) {}

    /// Produce the value. Called each time the host requests production.
    fn produce(&mut self, ctx: &mut Context) -> T;

    /// Called when demand for `T` drops to zero.
    fn deactivate(&mut self) {}
}

impl<T, F> Provider<T> for F
where
    T: 'static,
    F: FnMut() -> T,
{
    fn produce(&mut self, _ctx: &mut Context) -> T {
        self()
    }
}

pub(crate) struct ContextFn<F>(pub(crate) F);

impl<T, F> Provider<T> for ContextFn<F>
where
    T: 'static,
    F: FnMut(&mut Context) -> T,
{
    fn produce(&mut self, ctx: &mut Context) -> T {
        (self.0)(ctx)
    }
}

pub(crate) trait AnyProvider: Any {
    fn activate(&mut self);
    fn produce_erased(&mut self, ctx: &mut Context) -> Box<dyn Any>;
    fn deactivate(&mut self);
}

struct ProviderWrapper<T: 'static> {
    inner: Box<dyn Provider<T>>,
}

impl<T: 'static> AnyProvider for ProviderWrapper<T> {
    fn activate(&mut self) {
        self.inner.activate();
    }

    fn produce_erased(&mut self, ctx: &mut Context) -> Box<dyn Any> {
        Box::new(self.inner.produce(ctx))
    }

    fn deactivate(&mut self) {
        self.inner.deactivate();
    }
}

/// A typed handle identifying a registered provider node.
///
/// Obtained from [`PullGraph::provide`](crate::PullGraph::provide),
/// [`PullGraph::provide_with_deps`](crate::PullGraph::provide_with_deps), or
/// [`PullGraph::derive`](crate::PullGraph::derive). Used to declare dependencies
/// when registering derived providers.
pub struct ProviderHandle<T: 'static> {
    pub(crate) type_id: TypeId,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T: 'static> ProviderHandle<T> {
    pub(crate) fn new() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            _phantom: PhantomData,
        }
    }

    /// Returns the [`TypeId`] of the type this handle produces.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }
}

pub(crate) fn wrap_provider<T: 'static>(
    provider: impl Provider<T> + 'static,
) -> Box<dyn AnyProvider> {
    Box::new(ProviderWrapper {
        inner: Box::new(provider),
    })
}

/// Trait for dynamic dispatch of dependency handles.
///
/// Implemented for [`ProviderHandle<T>`](ProviderHandle) and `&dyn DepHandle`.
pub trait DepHandle {
    /// Returns the [`TypeId`] of the type this handle represents.
    fn type_id(&self) -> TypeId;
}

impl<T: 'static> DepHandle for ProviderHandle<T> {
    fn type_id(&self) -> TypeId {
        ProviderHandle::type_id(self)
    }
}

impl<T: DepHandle + ?Sized> DepHandle for &T {
    fn type_id(&self) -> TypeId {
        (**self).type_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn context_fn_produces_value() {
        let mut ctx_fn = ContextFn(|_ctx: &mut Context| 42i32);
        #[allow(clippy::arc_with_non_send_sync)]
        let mut ctx = Context::new(Arc::new(crate::inner::PullGraphInner::new()));
        assert_eq!(ctx_fn.produce(&mut ctx), 42);
    }

    #[test]
    fn zero_arg_closure_provider() {
        let mut f = || 42i32;
        #[allow(clippy::arc_with_non_send_sync)]
        let mut ctx = Context::new(Arc::new(crate::inner::PullGraphInner::new()));
        assert_eq!(f.produce(&mut ctx), 42);
    }

    #[test]
    fn dep_handle_ref_delegates() {
        let h = ProviderHandle::<i32>::new();
        let r: &ProviderHandle<i32> = &h;
        let dr: &dyn DepHandle = r;
        assert_eq!(dr.type_id(), TypeId::of::<i32>());
    }

    #[test]
    fn provider_handle_type_id() {
        let h = ProviderHandle::<i32>::new();
        assert_eq!(h.type_id(), TypeId::of::<i32>());
    }

    #[test]
    fn provider_trait_activate_deactivate() {
        struct LifecycleProvider(bool, bool);
        impl Provider<i32> for LifecycleProvider {
            fn activate(&mut self) {
                self.0 = true;
            }
            fn produce(&mut self, _ctx: &mut Context) -> i32 {
                1
            }
            fn deactivate(&mut self) {
                self.1 = true;
            }
        }
        let mut p = LifecycleProvider(false, false);
        p.activate();
        assert!(p.0);
        #[allow(clippy::arc_with_non_send_sync)]
        let mut ctx = Context::new(Arc::new(crate::inner::PullGraphInner::new()));
        assert_eq!(p.produce(&mut ctx), 1);
        p.deactivate();
        assert!(p.1);
    }
}
