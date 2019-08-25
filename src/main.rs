extern crate wormula;

use std::time::Instant;

use wormula::evaluator::*;
use wormula::parse::parse;

fn main() {
    let now = Instant::now();
    eprintln!("Compiling the wormula runtime library");
    let mut context = Context::new();
    eprintln!("Took {} ms", now.elapsed().as_millis());

    context.define_var("iterations");

    let now = Instant::now();
    let formula = "iterations == 0 or iterations == 1000000";
    eprintln!("Parsing the formula {}", formula);
    if let Ok((_, f1)) = parse(formula) {
        eprintln!("{} ms. AST: {:?}", now.elapsed().as_millis(), f1);
        eprintln!("Compiling formula+RTL");
        let now = Instant::now();
        let cf1 = context.compile(&f1);
        eprintln!("{} ms. Instantiating formula", now.elapsed().as_millis());
        let now = Instant::now();
        let if1 = cf1.instantiate();
        eprintln!(
            "{} ms. Running a loop until the formula returns true...",
            now.elapsed().as_millis()
        );
        // Note that we can subsequently refer to the variable directly,
        // no need to use the string key.
        let mut v = if1.get_variable("iterations").unwrap();
        let mut i = 0.0;
        loop {
            i += 1.0;
            v.set_f64(i);
            if if1.run() {
                break;
            }
        }
        println!("Did {} iterations in {} us.", i, now.elapsed().as_micros());

        let now = Instant::now();
        let mut i = 0.0;
        fn get_cond() -> (fn(f64) -> bool) {
            |c| c == 0.0 || c == 1000000.0
        };
        let check_cond = get_cond();
        loop {
            i += 1.0;
            if check_cond(i) {
                break;
            }
        }
        println!(
            "Did {} native iterations in {} us.",
            i,
            now.elapsed().as_micros()
        );
    }
}
