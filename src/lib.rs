//! Demand-driven computation for Rust.
//!
//! `pullgraph` lets components declare that they require a particular piece of
//! information. Producers of that information become active only while demand
//! exists. When demand disappears, production stops. Dependencies propagate
//! automatically.
//!
//! # Quick start
//!
//! ```ignore
//! use pullgraph::{PullGraph, Context};
//!
//! let pg = PullGraph::new();
//! let src = pg.provide(|| vec![1.0, 2.0, 3.0]).unwrap();
//! let mean = pg.derive([&src], |ctx: &mut Context| -> f64 {
//!     let v = ctx.get::<Vec<f64>>().unwrap();
//!     v.iter().sum::<f64>() / v.len() as f64
//! }).unwrap();
//!
//! let _demand = pg.want::<f64>().unwrap();
//! let mut ctx = pg.context();
//! ctx.produce::<f64>();
//! assert_eq!(ctx.get::<f64>(), Some(&2.0));
//! ```

pub mod context;
pub mod demand;
pub mod error;
pub mod graph;
pub mod inner;
pub mod provider;
pub mod pullgraph;

pub use context::Context;
pub use demand::Demand;
pub use error::PullGraphError;
pub use provider::{DepHandle, Provider, ProviderHandle};
pub use pullgraph::PullGraph;
