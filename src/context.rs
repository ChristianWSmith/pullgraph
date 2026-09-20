use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::inner::PullGraphInner;

/// Owns values produced during one execution step.
///
/// Created via [`PullGraph::context`](crate::PullGraph::context). Call
/// [`produce`](Context::produce) to produce values on demand, and
/// [`get`](Context::get) to retrieve previously produced values.
pub struct Context {
    pub(crate) inner: Arc<PullGraphInner>,
    pub(crate) produced: HashMap<TypeId, Box<dyn Any>>,
    pub(crate) producing: HashSet<TypeId>,
}

impl Context {
    pub(crate) fn new(inner: Arc<PullGraphInner>) -> Self {
        Self {
            inner,
            produced: HashMap::new(),
            producing: HashSet::new(),
        }
    }

    /// Produce a value of type `T` if it is demanded.
    ///
    /// Returns `Some(&T)` if the value was produced or was already cached in this
    /// context. Returns `None` if `T` has no active demand.
    ///
    /// Dependencies are produced automatically before `T`. Production is idempotent
    /// within a single `Context` — calling `produce::<T>()` twice invokes the
    /// provider only once.
    pub fn produce<T: 'static>(&mut self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.produce_inner(type_id);
        self.produced
            .get(&type_id)
            .and_then(|v| v.downcast_ref::<T>())
    }

    fn produce_inner(&mut self, type_id: TypeId) -> bool {
        if self.produced.contains_key(&type_id) {
            return true;
        }

        if self.producing.contains(&type_id) {
            panic!("recursive production of type {:?}", type_id);
        }

        let demanded = self.inner.is_demanded(type_id);
        if !demanded {
            return false;
        }

        let dependencies = self.inner.get_dependencies(type_id);

        self.producing.insert(type_id);

        for &dep_type_id in &dependencies {
            self.produce_inner(dep_type_id);
        }

        let result_box = {
            let provider_opt = self.inner.take_provider(type_id);
            match provider_opt {
                Some(Some(mut provider)) => {
                    let val = provider.produce_erased(self);
                    self.inner.restore_provider(type_id, provider);
                    Some(val)
                }
                Some(None) => {
                    panic!("provider for {:?} is already being produced", type_id);
                }
                None => None,
            }
        };

        self.producing.remove(&type_id);

        if let Some(val) = result_box {
            self.produced.insert(type_id, val);
            true
        } else {
            false
        }
    }

    /// Read a previously produced value without triggering production.
    ///
    /// Returns `Some(&T)` if `T` was produced in this context, `None` otherwise.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.produced
            .get(&type_id)
            .and_then(|v| v.downcast_ref::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PullGraph;
    use std::cell::Cell;
    use std::rc::Rc;

    struct SelfRecursive(Rc<Cell<bool>>);

    impl crate::provider::Provider<i32> for SelfRecursive {
        fn produce(&mut self, ctx: &mut Context) -> i32 {
            self.0.set(true);
            let _ = ctx.produce::<i32>();
            42
        }
    }

    #[test]
    #[should_panic(expected = "recursive production")]
    fn self_recursive_production_panics() {
        let pullgraph = PullGraph::new();
        let called = Rc::new(Cell::new(false));
        let counter = called.clone();
        let _h = pullgraph.provide(SelfRecursive(counter)).unwrap();
        let _d = pullgraph.want::<i32>().unwrap();
        let mut ctx = pullgraph.context();
        let _ = ctx.produce::<i32>();
    }

    #[test]
    fn produce_returns_none_for_undemanded_type() {
        let pullgraph = PullGraph::new();
        let _h = pullgraph.provide(|| 42i32).unwrap();
        let mut ctx = pullgraph.context();
        let result = ctx.produce::<i32>();
        assert!(result.is_none());
    }

    #[test]
    fn produce_returns_false_when_provider_removed() {
        let pullgraph = PullGraph::new();
        let _h = pullgraph.provide(|| 42i32).unwrap();
        let d = pullgraph.want::<i32>().unwrap();
        pullgraph.inner().remove_provider(TypeId::of::<i32>());
        let mut ctx = pullgraph.context();
        let result = ctx.produce::<i32>();
        assert!(result.is_none());
        std::mem::forget(d);
    }

    #[test]
    #[should_panic(expected = "already being produced")]
    fn null_provider_panics() {
        let pullgraph = PullGraph::new();
        let _h = pullgraph.provide(|| 42i32).unwrap();
        let _d = pullgraph.want::<i32>().unwrap();
        pullgraph.inner().null_provider(TypeId::of::<i32>());
        let mut ctx = pullgraph.context();
        let _ = ctx.produce::<i32>();
    }

    #[test]
    fn get_returns_none_for_unproduced_type() {
        let pullgraph = PullGraph::new();
        let ctx = pullgraph.context();
        assert!(ctx.get::<i32>().is_none());
    }

    #[test]
    fn produce_returns_true_for_cached_value() {
        let pullgraph = PullGraph::new();
        let _h = pullgraph.provide(|| 42i32).unwrap();
        let _d = pullgraph.want::<i32>().unwrap();
        let mut ctx = pullgraph.context();
        assert_eq!(ctx.produce::<i32>(), Some(&42));
        assert_eq!(ctx.produce::<i32>(), Some(&42));
    }

    #[test]
    fn produce_dependency_chain() {
        let pullgraph = PullGraph::new();
        let src = pullgraph.provide(|| 10i32).unwrap();
        let deps: &[&dyn crate::provider::DepHandle] = &[&src];
        let _derived = pullgraph.derive(deps, |ctx: &mut Context| -> i64 {
            ctx.get::<i32>().map(|&v| v as i64 * 2).unwrap_or(0)
        });
        let _d = pullgraph.want::<i64>().unwrap();
        let mut ctx = pullgraph.context();
        let val = ctx.produce::<i64>();
        assert_eq!(val, Some(&20));
    }

    struct ParentRemovingProvider(TypeId);

    impl crate::provider::Provider<i32> for ParentRemovingProvider {
        fn produce(&mut self, ctx: &mut Context) -> i32 {
            ctx.inner.remove_provider(self.0);
            42
        }
    }

    #[test]
    fn take_provider_returns_none_after_removal() {
        let pullgraph = PullGraph::new();
        let child_type = TypeId::of::<u64>();
        let _h = pullgraph
            .provide(ParentRemovingProvider(child_type))
            .unwrap();
        let deps: &[&dyn crate::provider::DepHandle] =
            &[&crate::provider::ProviderHandle::<i32>::new()];
        let _child = pullgraph
            .provide_with_deps(deps, |_: &mut Context| 0u64)
            .unwrap();
        let _d = pullgraph.want::<u64>().unwrap();
        let mut ctx = pullgraph.context();
        let result = ctx.produce::<u64>();
        assert!(result.is_none());
        std::mem::forget(_d);
    }
}
