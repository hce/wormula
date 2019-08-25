//! wormula -- parse once, evaluate often
//!
//! Wormula is a formula/condition parser that compiles the formulas to
//! webassembly for execution. It is intended to be used in similar
//! situations as a WHERE clause in SQL statements would be used, i.e.
//! where you parse a term once and evaluate it often.
//!
//! Instantiating wormula has a few seconds overhead, due to AOT
//! compilation of wasmer. This needs to be done once per thread,
//! not formula.
//!
//! Full usage example that does 1000000 iterations and then breaks:
//! ```
//!
//! let mut context = Context::new();
//! context.define_var("iterations");
//! let formula = "iterations == 0 or iterations == 1000000";
//! let f1 = parse(formula).unwrap().1;
//! let cf1 = context.compile(&f1);
//! let if1 = cf1.instantiate();
//! let mut i = 0.0;
//! let mut v = if1.get_variable("iterations").unwrap();
//! loop {
//!     i += 1.0;
//!     v.set_f64(i);
//!     if if1.run() {
//!         break;
//!     }
//! }
//! ```
#[macro_use]
extern crate wasmer_runtime;
extern crate nom;

#[allow(dead_code)]
pub mod evaluator;
#[allow(dead_code)]
pub mod parse;
pub mod term;
