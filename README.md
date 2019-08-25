Wormula is a formula/condition parser that compiles the formulas to
webassembly for execution. It is intended to be used in similar
situations as a WHERE clause in SQL statements would be used, i.e.
where you parse a term once and evaluate it often.

Instantiating wormula has a few seconds overhead, due to AOT
compilation of wasmer. See below for more details. wormula is probably
most useful in situations where the same formula must be applied many
times.

Contributions are welcome!

# Limitations

This project is in its earliest stage.

What works:

Operators: ==, !=, and, or
Data types: f64, strings and regexes.

Currently, the performance is not well at all. For each evaluation of a
compiled formula, multiple function calls need to be made. It may be more
feasible to let the formula evaluator operate on arrays to minimize the
overhead of calling to and communicating with the webassembly runtime.

# Build dependencies

You need to initialize and checkout the submodules:
 > git submodule init
 > git submodule update

You need the wasm32-unknown-unknown compilation target to build
the runtime library wormrtl:
 > rustup target add wasm32-unknown-unknown

# Building

Simply run "cargo build" after you have read the
"build dependencies" section.

# Example formulas

Compare a number:

    i == 1000000

Compare a string:

    license == "MIT"

Compare a regex and a number:
    name == /^Y/ and age == 48

# Example

As an example, we compile the formula "iterations == 1000000" and subsequently
run its evaluation in a loop, incrementing the variable "iterations" and break
the loop once the formula returns true.
(performance measuring code is omitted for clarity)

    // AOT compile the wasm runtime
    let mut context = Context::new();
    // define variables the formula has access to
    context.define_var("iterations");
    let formula = "iterations == 0 or iterations == 1000000";
    // parse the formula
    let f1 = parse(formula).unwrap().1;
    // compile the formula to webassembly
    let cf1 = context.compile(&f1);
    // AOT compile and instantiate the formula
    let if1 = cf1.instantiate();
    let mut i = 0.0;
    // Get a reference to the variable by string key
    let mut v = if1.get_variable("iterations").unwrap();
    loop {
        i += 1.0;
        v.set_f64(i);
        if if1.run() {
            break;
        }
    }

The following output is generated:

    Compiling the wormula runtime library
    Took 1048 ms
    Parsing the formula "iterations == 1000000"
    0 ms. AST: Eq(Variable("iterations"), Float(1000000.0))
    Compiling formula+RTL
    2 ms. Instantiating formula
    0 ms. Running a loop until the formula returns true...
    Did 1000000 iterations in 2462 ms.
    

# Implementation notes

Instantiating a wormula instance takes a few seconds, as wasmer
JIT/AOT compiles the wormula runtime library to machine code. The
wormula RTL is also written in rust and makes much use of rust's
standard libraries.

Ironically, due to the use of wasmer to AOT compile the webassembly part, it is
currently not possible to use this library for wasm targets. This will require
the communication between rust and javascript. It is planned to add support at
some point for this.

