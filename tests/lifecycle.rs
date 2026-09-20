use pullgraph::{Context, Provider, PullGraph};
use std::cell::Cell;
use std::rc::Rc;

type Lifecycles = (Rc<Cell<bool>>, Rc<Cell<bool>>, Rc<Cell<u32>>);

struct Counter {
    activated: Rc<Cell<bool>>,
    deactivated: Rc<Cell<bool>>,
    produce_count: Rc<Cell<u32>>,
}

impl Counter {
    fn new() -> (Self, Lifecycles) {
        let activated = Rc::new(Cell::new(false));
        let deactivated = Rc::new(Cell::new(false));
        let produce_count = Rc::new(Cell::new(0));
        (
            Self {
                activated: activated.clone(),
                deactivated: deactivated.clone(),
                produce_count: produce_count.clone(),
            },
            (activated, deactivated, produce_count),
        )
    }
}

impl Provider<i32> for Counter {
    fn activate(&mut self) {
        self.activated.set(true);
    }

    fn produce(&mut self, _ctx: &mut Context) -> i32 {
        self.produce_count.set(self.produce_count.get() + 1);
        42
    }

    fn deactivate(&mut self) {
        self.deactivated.set(true);
    }
}

#[test]
fn activate_called_on_first_demand() {
    let pullgraph = PullGraph::new();
    let (counter, (activated, _deactivated, _produce_count)) = Counter::new();
    let _h = pullgraph.provide(counter).unwrap();
    let _demand = pullgraph.want::<i32>().unwrap();
    assert!(activated.get());
}

#[test]
fn deactivate_called_on_last_drop() {
    let pullgraph = PullGraph::new();
    let (counter, (_activated, deactivated, _produce_count)) = Counter::new();
    let _h = pullgraph.provide(counter).unwrap();
    let d1 = pullgraph.want::<i32>().unwrap();
    let d2 = pullgraph.want::<i32>().unwrap();
    drop(d1);
    assert!(!deactivated.get());
    drop(d2);
    assert!(deactivated.get());
}

#[test]
fn produce_count_tracked() {
    let pullgraph = PullGraph::new();
    let (counter, (_activated, _deactivated, produce_count)) = Counter::new();
    let _h = pullgraph.provide(counter).unwrap();
    let _demand = pullgraph.want::<i32>().unwrap();

    let mut ctx = pullgraph.context();
    ctx.produce::<i32>();
    assert_eq!(produce_count.get(), 1);
    ctx.produce::<i32>();
    assert_eq!(produce_count.get(), 1);
}
