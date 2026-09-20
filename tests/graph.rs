use pullgraph::PullGraph;
use pullgraph::provider::DepHandle;

#[test]
fn activation_propagation() {
    let pullgraph = PullGraph::new();
    let a = pullgraph.provide(|| 1i32).unwrap();
    let deps: &[&dyn DepHandle] = &[&a];
    let _b: pullgraph::ProviderHandle<i64> = pullgraph
        .derive(deps, |ctx| -> i64 {
            ctx.get::<i32>().map(|&v| v as i64 + 1).unwrap_or(0)
        })
        .unwrap();

    assert!(!pullgraph.is_demanded::<i32>());
    assert!(!pullgraph.is_demanded::<i64>());

    let _demand = pullgraph.want::<i64>().unwrap();

    assert!(pullgraph.is_demanded::<i32>());
    assert!(pullgraph.is_demanded::<i64>());
}

#[test]
fn deactivation_unwinding() {
    let pullgraph = PullGraph::new();
    let a = pullgraph.provide(|| 1i32).unwrap();
    let deps: &[&dyn DepHandle] = &[&a];
    let _b: pullgraph::ProviderHandle<i64> = pullgraph
        .derive(deps, |ctx| -> i64 {
            ctx.get::<i32>().map(|&v| v as i64 + 1).unwrap_or(0)
        })
        .unwrap();

    let demand = pullgraph.want::<i64>().unwrap();
    assert!(pullgraph.is_demanded::<i32>());
    assert!(pullgraph.is_demanded::<i64>());

    drop(demand);
    assert!(!pullgraph.is_demanded::<i32>());
    assert!(!pullgraph.is_demanded::<i64>());
}

#[test]
fn cycle_detection() {
    let pullgraph = PullGraph::new();
    let a = pullgraph.provide(|| 1i32).unwrap();
    let deps_a: &[&dyn DepHandle] = &[&a];
    let b_result = pullgraph.derive(deps_a, |ctx| -> i64 {
        ctx.get::<i32>().map(|&v| v as i64).unwrap_or(0)
    });
    assert!(b_result.is_ok());
    let b = b_result.unwrap();

    let deps_b: &[&dyn DepHandle] = &[&b];
    let result = pullgraph.provide_with_deps(deps_b, |ctx| -> i32 {
        ctx.get::<i64>().map(|&v| v as i32).unwrap_or(0)
    });
    assert!(result.is_err());
}
