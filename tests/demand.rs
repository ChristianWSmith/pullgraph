use pullgraph::{Demand, PullGraph};
use std::any::TypeId;

#[test]
fn demand_creates_for_registered_type() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let _demand: Demand<i32> = pullgraph.want().unwrap();
    assert!(pullgraph.is_demanded::<i32>());
}

#[test]
fn demand_increments_count() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let _d1: Demand<i32> = pullgraph.want().unwrap();
    assert_eq!(pullgraph.demand_count::<i32>(), 1);
    let _d2: Demand<i32> = pullgraph.want().unwrap();
    assert_eq!(pullgraph.demand_count::<i32>(), 2);
}

#[test]
fn demand_decrements_on_drop() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let d1: Demand<i32> = pullgraph.want().unwrap();
    let d2: Demand<i32> = pullgraph.want().unwrap();
    assert_eq!(pullgraph.demand_count::<i32>(), 2);
    drop(d1);
    assert_eq!(pullgraph.demand_count::<i32>(), 1);
    drop(d2);
    assert_eq!(pullgraph.demand_count::<i32>(), 0);
    assert!(!pullgraph.is_demanded::<i32>());
}

#[test]
fn demand_clone_creates_independent_reference() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let d1: Demand<i32> = pullgraph.want().unwrap();
    let d2 = d1.clone();
    assert_eq!(pullgraph.demand_count::<i32>(), 2);
    drop(d1);
    assert_eq!(pullgraph.demand_count::<i32>(), 1);
    drop(d2);
    assert_eq!(pullgraph.demand_count::<i32>(), 0);
}

#[test]
fn demand_type_id_matches() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let demand: Demand<i32> = pullgraph.want().unwrap();
    assert_eq!(demand.type_id(), TypeId::of::<i32>());
}

#[test]
fn want_error_for_unregistered_type() {
    let pullgraph = PullGraph::new();
    let result = pullgraph.want::<i32>();
    assert!(result.is_err());
}
