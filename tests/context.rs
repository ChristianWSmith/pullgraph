use pullgraph::provider::DepHandle;
use pullgraph::{Context, Provider, PullGraph};
use std::cell::Cell;
use std::rc::Rc;

struct StepCounter(Rc<Cell<u32>>);

impl Provider<i32> for StepCounter {
    fn produce(&mut self, _ctx: &mut Context) -> i32 {
        self.0.set(self.0.get() + 1);
        self.0.get() as i32
    }
}

#[test]
fn context_idempotency() {
    let pullgraph = PullGraph::new();
    let call_count = Rc::new(Cell::new(0));
    let counter = StepCounter(call_count.clone());
    let _h = pullgraph.provide(counter).unwrap();
    let _demand = pullgraph.want::<i32>().unwrap();

    let mut ctx = pullgraph.context();
    let val1 = ctx.produce::<i32>();
    assert_eq!(val1, Some(&1));
    assert_eq!(call_count.get(), 1);

    let val2 = ctx.produce::<i32>();
    assert_eq!(val2, Some(&1));
    assert_eq!(call_count.get(), 1);
}

#[test]
fn new_context_allows_reproduction() {
    let pullgraph = PullGraph::new();
    let call_count = Rc::new(Cell::new(0));
    let counter = StepCounter(call_count.clone());
    let _h = pullgraph.provide(counter).unwrap();
    let _demand = pullgraph.want::<i32>().unwrap();

    let mut ctx1 = pullgraph.context();
    ctx1.produce::<i32>();
    assert_eq!(call_count.get(), 1);

    let mut ctx2 = pullgraph.context();
    let val = ctx2.produce::<i32>();
    assert_eq!(val, Some(&2));
    assert_eq!(call_count.get(), 2);
}

#[test]
fn produce_none_when_undemanded() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();

    let mut ctx = pullgraph.context();
    let val = ctx.produce::<i32>();
    assert!(val.is_none());
}

#[test]
fn get_without_produce() {
    let pullgraph = PullGraph::new();
    let _h = pullgraph.provide(|| 42i32).unwrap();
    let _demand = pullgraph.want::<i32>().unwrap();

    let mut ctx = pullgraph.context();
    ctx.produce::<i32>();
    let val = ctx.get::<i32>();
    assert_eq!(val, Some(&42));
}

#[test]
fn dependency_production_in_context() {
    let pullgraph = PullGraph::new();
    let source = pullgraph.provide(|| 10i32).unwrap();
    let deps: &[&dyn DepHandle] = &[&source];
    let _derived = pullgraph
        .derive(deps, |ctx: &mut Context| -> i64 {
            ctx.get::<i32>().map(|&v| v as i64 * 3).unwrap_or(0)
        })
        .unwrap();

    let _demand = pullgraph.want::<i64>().unwrap();

    let mut ctx = pullgraph.context();
    let val = ctx.produce::<i64>();
    assert_eq!(val, Some(&30));
}
