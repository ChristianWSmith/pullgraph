use pullgraph::provider::DepHandle;
use pullgraph::{Context, PullGraph};

#[test]
fn multiple_demanders_share_production() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let _d1 = pullgraph.want::<i32>().unwrap();
    let _d2 = pullgraph.want::<i32>().unwrap();
    assert_eq!(pullgraph.demand_count::<i32>(), 2);

    let mut ctx = pullgraph.context();
    let val = ctx.produce::<i32>();
    assert_eq!(val, Some(&42));

    let val2 = ctx.produce::<i32>();
    assert_eq!(val2, Some(&42));
}

#[test]
fn shared_derived_production() {
    let pullgraph = PullGraph::new();
    let source = pullgraph.provide(|| 10i32).unwrap();
    let deps: &[&dyn DepHandle] = &[&source];
    let _derived = pullgraph
        .derive(deps, |ctx: &mut Context| -> i64 {
            ctx.get::<i32>().map(|&v| v as i64 * 2).unwrap_or(0)
        })
        .unwrap();

    let _d1 = pullgraph.want::<i64>().unwrap();
    let _d2 = pullgraph.want::<i64>().unwrap();

    let mut ctx = pullgraph.context();
    let val = ctx.produce::<i64>();
    assert_eq!(val, Some(&20));
}
