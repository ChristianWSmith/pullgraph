# pullgraph

Demand-driven computation for Rust.

`pullgraph` lets components declare that they require a particular piece of information. Producers of that information become active only while demand exists. When demand disappears, production stops. Dependencies propagate automatically — demanding a derived value activates everything required to produce it.

```rust
use pullgraph::{PullGraph, Context};

struct Input(Vec<f64>);
struct Stats { mean: f64 }
struct Report(String);

let pg = PullGraph::new();

let input = pg.provide(|| Input(vec![1.0, 2.0, 3.0])).unwrap();
let stats = pg.derive([&input], |ctx: &mut Context| {
    let input = ctx.get::<Input>().unwrap();
    Stats { mean: input.0.iter().sum::<f64>() / input.0.len() as f64 }
}).unwrap();
let report = pg.derive([&stats], |ctx: &mut Context| {
    let stats = ctx.get::<Stats>().unwrap();
    Report(format!("mean: {}", stats.mean))
}).unwrap();

// Demand triggers activation
let _demand = pg.want::<Report>().unwrap();

// Host controls when production happens
let mut ctx = pg.context();
ctx.produce::<Report>();
let report = ctx.get::<Report>().unwrap();
assert_eq!(report.0, "mean: 2");

// When demand drops, providers deactivate
drop(_demand);
```

## Core Concepts

**`PullGraph`** — the central registry. Register providers, create demand, inspect state.

**`ProviderHandle<T>`** — a typed handle identifying a registered provider for type `T`. Used to declare dependencies.

**`Demand<T>`** — an RAII handle representing active demand for `T`. Dropping it releases demand. Cloning acquires an independent reference.

**`Context`** — owns values produced during one execution step. Created from `PullGraph` via `pg.context()`.

**`Provider<T>`** — trait for anything that produces a `T`. Implement `produce` (and optionally `activate`/`deactivate`).

## Usage

### Registering providers

```rust
use pullgraph::{PullGraph, Provider, Context};

let pg = PullGraph::new();

// Simple provider — closure with no arguments
let handle = pg.provide(|| 42i32).unwrap();

// Provider with dependencies — closure receives &mut Context
let src = pg.provide(|| vec![1, 2, 3]).unwrap();
let sum = pg.provide_with_deps([&src], |ctx: &mut Context| -> i32 {
    ctx.get::<Vec<i32>>().unwrap().iter().sum()
}).unwrap();

// Derived provider — sugar for provide_with_deps + closure
let doubled = pg.derive([&sum], |ctx: &mut Context| -> i64 {
    *ctx.get::<i32>().unwrap() as i64 * 2
}).unwrap();
```

### Creating demand

```rust
let demand = pg.want::<i32>().unwrap();  // returns error if no provider
let demand2 = demand.clone();              // independent reference count

assert!(pg.is_demanded::<i32>());
assert_eq!(pg.demand_count::<i32>(), 2);

drop(demand);
assert_eq!(pg.demand_count::<i32>(), 1);

drop(demand2);
assert!(!pg.is_demanded::<i32>());
```

### Production

```rust
let src = pg.provide(|| 10i32).unwrap();
let derived = pg.derive([&src], |ctx: &mut Context| -> i64 {
    ctx.get::<i32>().map(|&v| v as i64 * 2).unwrap_or(0)
}).unwrap();
let _demand = pg.want::<i64>().unwrap();

// Create a context for one execution step
let mut ctx = pg.context();

// produce() returns Option<&T> — None if undemanded
let val = ctx.produce::<i64>();
assert_eq!(val, Some(&20));

// get() reads without producing
let val = ctx.get::<i64>();
assert_eq!(val, Some(&20));

// produce() is idempotent within a context
ctx.produce::<i64>();  // provider not called again
```

### Lifecycle hooks

```rust
use std::cell::Cell;
use std::rc::Rc;

let activated = Rc::new(Cell::new(false));
let deactivated = Rc::new(Cell::new(false));

struct MyProvider { a: Rc<Cell<bool>>, b: Rc<Cell<bool>> }

impl Provider<i32> for MyProvider {
    fn activate(&mut self) { self.a.set(true); }
    fn produce(&mut self, _ctx: &mut Context) -> i32 { 42 }
    fn deactivate(&mut self) { self.b.set(true); }
}

let pg = PullGraph::new();
let _h = pg.provide(MyProvider { a: activated.clone(), b: deactivated.clone() }).unwrap();

let d = pg.want::<i32>().unwrap();
assert!(activated.get());      // activate called on first demand

let mut ctx = pg.context();
ctx.produce::<i32>();          // produce called when demanded

drop(d);
assert!(deactivated.get());    // deactivate called when demand reaches zero
```

### Diamond dependencies

```rust
let pg = PullGraph::new();
let a = pg.provide(|| 1i32).unwrap();
let b = pg.derive([&a], |ctx: &mut Context| -> i64 {
    ctx.get::<i32>().map(|&v| v as i64 + 10).unwrap_or(0)
}).unwrap();
let c = pg.derive([&a], |ctx: &mut Context| -> i64 {
    ctx.get::<i32>().map(|&v| v as i64 + 20).unwrap_or(0)
}).unwrap();

struct Diamond(i64, i64);
let d = pg.derive([&b, &c], |ctx: &mut Context| {
    Diamond(*ctx.get::<i64>().unwrap(), *ctx.get::<i64>().unwrap())
}).unwrap();

let _demand = pg.want::<Diamond>().unwrap();
// Demanding D activates B, C, and A (with demand_count = 2)
```

## Design Principles

- **No demand, no cost.** A provider with zero active demand performs no mandatory production.
- **Host-controlled execution.** `PullGraph` determines *what* is demanded. Your code determines *when* production happens.
- **RAII demand.** `Demand<T>` is an owned Rust value. Dropping it releases demand. No explicit start/stop.
- **Shared production.** Multiple consumers of the same type share a single provider execution.
- **Recursive activation.** Demanding a derived value automatically activates its entire dependency chain.
- **Cycle detection.** Dependency cycles are caught at registration time, not at runtime.

## API Reference

| Type | Description |
|------|-------------|
| `PullGraph` | Central registry. Create with `PullGraph::new()`. |
| `ProviderHandle<T>` | Typed key identifying a registered provider. |
| `Demand<T>` | RAII demand handle. Clonable, independently reference-counted. |
| `Context` | Owns produced values for one execution step. |
| `Provider<T>` | Trait: `activate()`, `produce(&mut Context) -> T`, `deactivate()`. |
| `PullGraphError` | Error enum: `AlreadyRegistered`, `NoProvider`, `DependencyCycle`. |

| Method | Description |
|--------|-------------|
| `PullGraph::provide(p)` | Register a provider for `T`. Returns `ProviderHandle<T>`. |
| `PullGraph::provide_with_deps(deps, p)` | Register with dependencies. |
| `PullGraph::derive(deps, \|ctx\| ...)` | Shorthand for dependency + transform. |
| `PullGraph::want::<T>()` | Create demand. Returns `Demand<T>`. |
| `PullGraph::is_demanded::<T>()` | Check if any demand exists. |
| `PullGraph::demand_count::<T>()` | Get current demand count. |
| `PullGraph::context()` | Create a `Context` for production. |
| `Context::produce::<T>()` | Produce `T` if demanded. Returns `Option<&T>`. |
| `Context::get::<T>()` | Read a previously produced value. Returns `Option<&T>`. |

## License

MIT
