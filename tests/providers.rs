use pullgraph::provider::DepHandle;
use pullgraph::{PullGraph, PullGraphError};

#[test]
fn provide_registers_provider() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let _d = pullgraph.want::<i32>().unwrap();
    assert!(pullgraph.is_demanded::<i32>());
}

#[test]
fn provide_duplicate_returns_error() {
    let pullgraph = PullGraph::new();
    let _h1 = pullgraph.provide(|| 42i32).unwrap();
    let result = pullgraph.provide(|| 43i32);
    assert!(matches!(result, Err(PullGraphError::AlreadyRegistered(_))));
}

#[test]
fn provide_with_deps_registers() {
    let pullgraph = PullGraph::new();
    let h1 = pullgraph.provide(|| 1i32).unwrap();
    let deps: &[&dyn DepHandle] = &[&h1];
    let _h2 = pullgraph
        .provide_with_deps(deps, |ctx| -> i64 {
            ctx.get::<i32>().map(|&v| v as i64).unwrap_or(0)
        })
        .unwrap();
    let _d = pullgraph.want::<i64>().unwrap();
    assert!(pullgraph.is_demanded::<i64>());
}

#[test]
fn derive_registers_provider() {
    let pullgraph = PullGraph::new();
    let h1 = pullgraph.provide(|| 1i32).unwrap();
    let deps: &[&dyn DepHandle] = &[&h1];
    let _h2 = pullgraph
        .derive(deps, |ctx| -> i64 {
            ctx.get::<i32>().map(|&v| v as i64).unwrap_or(0)
        })
        .unwrap();
    let _d = pullgraph.want::<i64>().unwrap();
    assert!(pullgraph.is_demanded::<i64>());
}
