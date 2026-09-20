use pullgraph::PullGraph;
use pullgraph::provider::DepHandle;

struct Source(i32);
struct Middle(i64);
struct Other(i64);
struct Derived(String);

#[test]
fn derive_chain_produces_correctly() {
    let pullgraph = PullGraph::new();
    let source = pullgraph.provide(|| Source(42)).unwrap();
    let deps: &[&dyn DepHandle] = &[&source];
    let middle = pullgraph
        .derive(deps, |ctx| {
            let s = ctx.get::<Source>().unwrap();
            Middle(s.0 as i64 * 2)
        })
        .unwrap();
    let deps2: &[&dyn DepHandle] = &[&middle];
    let _derived = pullgraph
        .derive(deps2, |ctx| {
            let m = ctx.get::<Middle>().unwrap();
            Derived(format!("value: {}", m.0))
        })
        .unwrap();

    let _demand = pullgraph.want::<Derived>().unwrap();

    let mut ctx = pullgraph.context();
    let val = ctx.produce::<Derived>();
    assert!(val.is_some());
    assert_eq!(val.unwrap().0, "value: 84");
}

#[test]
fn derive_only_produces_demanded() {
    let pullgraph = PullGraph::new();
    let source = pullgraph.provide(|| Source(42)).unwrap();
    let deps: &[&dyn DepHandle] = &[&source];
    let _middle = pullgraph
        .derive(deps, |ctx| {
            let s = ctx.get::<Source>().unwrap();
            Middle(s.0 as i64 * 2)
        })
        .unwrap();

    let mut ctx = pullgraph.context();
    let val = ctx.produce::<Source>();
    assert!(val.is_none());
}

#[test]
fn diamond_dependency() {
    let pullgraph = PullGraph::new();
    let a = pullgraph.provide(|| Source(1)).unwrap();
    let deps_a: &[&dyn DepHandle] = &[&a];
    let b = pullgraph
        .derive(deps_a, |ctx| {
            let s = ctx.get::<Source>().unwrap();
            Middle(s.0 as i64 + 10)
        })
        .unwrap();
    let deps_a2: &[&dyn DepHandle] = &[&a];
    let c = pullgraph
        .derive(deps_a2, |ctx| {
            let s = ctx.get::<Source>().unwrap();
            Other(s.0 as i64 + 20)
        })
        .unwrap();

    #[allow(dead_code)]
    struct Diamond(i64, i64);
    let deps_bc: &[&dyn DepHandle] = &[&b, &c];
    let _d = pullgraph
        .derive(deps_bc, |ctx| {
            let b = ctx.get::<Middle>().unwrap();
            let c = ctx.get::<Other>().unwrap();
            Diamond(b.0, c.0)
        })
        .unwrap();

    let demand = pullgraph.want::<Diamond>().unwrap();

    assert!(pullgraph.is_demanded::<Source>());
    assert!(pullgraph.is_demanded::<Middle>());
    assert!(pullgraph.is_demanded::<Other>());
    assert!(pullgraph.is_demanded::<Diamond>());

    let mut ctx = pullgraph.context();
    let val = ctx.produce::<Diamond>();
    assert!(val.is_some());

    drop(demand);
    assert!(!pullgraph.is_demanded::<Diamond>());
}
