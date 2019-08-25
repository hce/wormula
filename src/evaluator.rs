use parity_wasm::builder;
use parity_wasm::elements;
use parity_wasm::elements::*;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

use crate::term::*;

static FUN_MAKE_STATE: u32 = 0;
static FUN_FREE_STATE: u32 = 1;
static FUN_MAKE_I64: u32 = 2;
static FUN_MAKE_F64: u32 = 3;
static FUN_RTL_EQ: u32 = 4;
static FUN_RTL_GET_BOOL: u32 = 5;
static FUN_RTL_AND: u32 = 6;
static FUN_RTL_OR: u32 = 7;
static FUN_RTL_NOT: u32 = 8;
static FUN_ALLOC_STRING: u32 = 9;
static FUN_GET_STRING_BUF: u32 = 10;
static FUN_MAKE_STRING: u32 = 11;
static FUN_MAKE_REGEX: u32 = 12;

/// Context represents an evaluation context that can be used to
/// parse and execute one or more formulas.
pub struct Context<'a> {
    rtl_module: wasmer_runtime::Module,
    variables: HashMap<&'a str, i64>,
    instructions: Vec<Instruction>,
    idx_state: Option<u32>,
    idx_result: Option<u32>,
    idx_string: Option<u32>,
    idx_mem_buf_ptr: Option<u32>,
    idx_var_result: Option<u32>,
    locals: i64,
    reserved_slots: i64,
}

/// Variable represents a variable of a specific instance of a formula.
pub struct Variable<'b, 'a: 'b> {
    idx: i64,
    ct: &'b InstantiatedTerm<'a, 'b>,
}

impl<'a, 'b> Variable<'a, 'b> {
    /// Sets the value of the references variable to ```string```
    pub fn set_string(&mut self, string: &str) {
        let string_bytes = string.as_bytes();
        let alloc_buffer = self
            .ct
            .alloc_string_call
            .call(string_bytes.len() as i32)
            .expect("call alloc_string");
        let ptr = self
            .ct
            .get_string_buf_call
            .call(alloc_buffer)
            .expect("call get_string_buf") as usize;
        let view = self.ct.memory.view::<u8>();
        for i in 0..string_bytes.len() {
            view[ptr + i].set(string_bytes[i]);
        }
        let s = self
            .ct
            .make_string_call
            .call(
                self.ct.ct.fm_init_res,
                self.idx,
                alloc_buffer,
                string_bytes.len() as i32,
            )
            .expect("make_string");
        if s != self.idx {
            eprintln!("s != self.idx, what gives?");
        }
    }

    /// Sets the value of the references variable to ```intval```
    pub fn set_i64(&mut self, intval: i64) {
        self.ct
            .make_i64_call
            .call(self.ct.ct.fm_init_res, self.idx, intval)
            .expect("call make_i64");
    }

    /// Sets the value of the references variable to ```fval```
    pub fn set_f64(&mut self, fval: f64) {
        self.ct
            .make_f64_call
            .call(self.ct.ct.fm_init_res, self.idx, fval)
            .expect("call make_f64");
    }
}

/// CompiledTerm represents a compiled formula
pub struct CompiledTerm<'b, 'a: 'b> {
    //    state: RuntimeValue,
    //    module_instance: ModuleRef,
    //    library_instance: ModuleRef,
    //    mem: ExternVal,
    //    alloc_string: ExternVal,
    //    get_string_buf: ExternVal,
    //    make_string: ExternVal,
    rtl_module_instance: Rc<wasmer_runtime::Instance>,
    fm_import_object: wasmer_runtime::ImportObject,
    formula_module_instance: wasmer_runtime::Instance,
    fm_init_res: i32,
    context: &'b Context<'a>,
}

/// InstantiatedTerm represents a loaded (i.e., AOT compiled)
/// formula with associated variables
pub struct InstantiatedTerm<'b, 'a: 'b> {
    ct: &'b CompiledTerm<'b, 'a>,
    make_i64_call: wasmer_runtime::Func<'b, (i32, i64, i64), i64>,
    make_f64_call: wasmer_runtime::Func<'b, (i32, i64, f64), i64>,
    alloc_string_call: wasmer_runtime::Func<'b, (i32), (i32)>,
    get_string_buf_call: wasmer_runtime::Func<'b, (i32), (i32)>,
    make_string_call: wasmer_runtime::Func<'b, (i32, i64, i32, i32), (i64)>,
    eval_call: wasmer_runtime::Func<'b, i32, i32>,
    memory: &'b wasmer_runtime::Memory,
}

fn print_str(ctx: &mut wasmer_runtime::Ctx, ptr: u32, len: u32) {
    let mv = ctx.memory(0).view::<u8>();
    let mut chars = Vec::with_capacity(len as usize);
    for i in ptr..ptr + len {
        chars.push(mv[i as usize].get());
    }
    let s = String::from_utf8_lossy(chars.as_slice());
    println!("WASM: {}", s);
}

impl<'a> Context<'a> {
    /// Create a new context.
    ///
    /// A context is used to parse, compile and instantiate one or
    /// more formulas.
    ///
    /// Creating a context AOT compiles the runtime library,
    /// which takes a few seconds to complete.
    ///
    /// A single context can be used to handle independent formulas.
    /// All formulas share the names of the variables declared
    /// per context. This does not mean that the values of variables
    /// are shared between formulas.
    pub fn new() -> Context<'a> {
        let variables = HashMap::new();
        let instructions = Vec::new();
        let idx_state = None;
        let idx_result = None;
        let idx_string = None;
        let idx_mem_buf_ptr = None;
        let idx_var_result = None;
        let locals = 1; /* Need to start counting at 1! */
        let reserved_slots = 1000;

        let lib_wasm_rtl =
            include_bytes!("../wormrtl/target/wasm32-unknown-unknown/release/wormrtl.wasm");
        let rtl_module = wasmer_runtime::compile(lib_wasm_rtl).expect("wormrtl.wasm module");

        Context {
            rtl_module,
            variables,
            instructions,
            idx_state,
            idx_result,
            idx_string,
            idx_mem_buf_ptr,
            idx_var_result,
            locals,
            reserved_slots,
        }
    }

    /// All variables referenced by formulas compiled within a
    /// context need to be previously declared using the define_var
    /// function. Since types are associated at runtime, there
    /// is no need to declare the type of the variable here.
    pub fn define_var<'b>(&'b mut self, var_name: &'a str) {
        let var_num = self.locals;
        self.locals += 1;
        self.variables.insert(var_name, var_num);
    }

    fn int_build_loader<'d>(&mut self, t: &Term) -> Term<'d> {
        match &t {
            &Term::Int(intval) => {
                let my_local_idx = self.locals;
                self.locals += 1;
                self.instructions
                    .push(Instruction::GetLocal(self.idx_state.expect("P4")));
                self.instructions.push(Instruction::I64Const(my_local_idx));
                self.instructions.push(Instruction::I64Const(*intval));
                self.instructions.push(Instruction::Call(FUN_MAKE_I64));
                self.instructions.push(Instruction::Drop);
                Term::LoadedTerm(my_local_idx)
            }
            &Term::Float(fval) => {
                let my_local_idx = self.locals;
                self.locals += 1;
                self.instructions
                    .push(Instruction::GetLocal(self.idx_state.expect("P4")));
                self.instructions.push(Instruction::I64Const(my_local_idx));
                self.instructions
                    .push(Instruction::F64Const(fval.to_bits()));
                self.instructions.push(Instruction::Call(FUN_MAKE_F64));
                self.instructions.push(Instruction::Drop);
                Term::LoadedTerm(my_local_idx)
            }
            &Term::String(sval) => {
                // TODO: max string length check as i32 != usize !
                let string_bytes = sval.as_bytes();
                let my_local_idx = self.locals;
                self.locals += 1;

                // Allocate enough bytes for the string on the rtl's heap
                // +Get the string's temporary buffer
                self.instructions
                    .push(Instruction::I32Const(string_bytes.len() as i32));
                self.instructions.push(Instruction::Call(FUN_ALLOC_STRING));
                self.instructions
                    .push(Instruction::TeeLocal(self.idx_string.expect("P5")));
                self.instructions
                    .push(Instruction::Call(FUN_GET_STRING_BUF));
                self.instructions
                    .push(Instruction::SetLocal(self.idx_mem_buf_ptr.expect("P6")));

                // Copy the string's bytes to the rtl's heap
                // (TODO: find a more efficient way to do so)
                let mut i = 0;
                for b in string_bytes {
                    self.instructions
                        .push(Instruction::GetLocal(self.idx_mem_buf_ptr.expect("P6")));
                    self.instructions.push(Instruction::I32Const(*b as i32));
                    self.instructions.push(Instruction::I32Store8(0, i));
                    i += 1;
                }

                // Make a string out of the bytes :-)
                self.instructions
                    .push(Instruction::GetLocal(self.idx_state.expect("P4")));
                self.instructions.push(Instruction::I64Const(my_local_idx));
                self.instructions
                    .push(Instruction::GetLocal(self.idx_string.expect("P5")));
                self.instructions
                    .push(Instruction::I32Const(string_bytes.len() as i32));
                self.instructions.push(Instruction::Call(FUN_MAKE_STRING));
                self.instructions.push(Instruction::Drop);
                Term::LoadedTerm(my_local_idx)
            }
            &Term::Regex(rval) => {
                // TODO: max string length check as i32 != usize !
                let string_bytes = rval.as_bytes();
                let my_local_idx = self.locals;
                self.locals += 1;

                // Allocate enough bytes for the string on the rtl's heap
                // +Get the string's temporary buffer
                self.instructions
                    .push(Instruction::I32Const(string_bytes.len() as i32));
                self.instructions.push(Instruction::Call(FUN_ALLOC_STRING));
                self.instructions
                    .push(Instruction::TeeLocal(self.idx_string.expect("P5")));
                self.instructions
                    .push(Instruction::Call(FUN_GET_STRING_BUF));
                self.instructions
                    .push(Instruction::SetLocal(self.idx_mem_buf_ptr.expect("P6")));

                // Copy the string's bytes to the rtl's heap
                // (TODO: find a more efficient way to do so)
                let mut i = 0;
                for b in string_bytes {
                    self.instructions
                        .push(Instruction::GetLocal(self.idx_mem_buf_ptr.expect("P6")));
                    self.instructions.push(Instruction::I32Const(*b as i32));
                    self.instructions.push(Instruction::I32Store8(0, i));
                    i += 1;
                }

                let regex_flags = 0; // TODO

                // Make a string out of the bytes :-)
                self.instructions
                    .push(Instruction::GetLocal(self.idx_state.expect("P4")));
                self.instructions.push(Instruction::I64Const(my_local_idx));
                self.instructions.push(Instruction::I64Const(regex_flags));
                self.instructions
                    .push(Instruction::GetLocal(self.idx_string.expect("P5")));
                self.instructions
                    .push(Instruction::I32Const(string_bytes.len() as i32));
                self.instructions.push(Instruction::Call(FUN_MAKE_REGEX));
                self.instructions.push(Instruction::Drop);
                Term::LoadedTerm(my_local_idx)
            }
            &Term::Variable(var_name) => match self.variables.get(*var_name) {
                Some(v) => Term::LoadedTerm(*v),
                None => Term::LoadedTerm(-1),
            },
            &Term::LoadedTerm(_) => None.expect("Cannot double-build loader"),
            &Term::And(t1, t2) => {
                let t1_d = self.int_build_loader(t1);
                let t2_d = self.int_build_loader(t2);
                Term::And(Box::new(t1_d), Box::new(t2_d))
            }
            &Term::Not(t) => Term::Not(Box::new(self.int_build_loader(t))),
            &Term::Or(t1, t2) => {
                let t1_d = self.int_build_loader(t1);
                let t2_d = self.int_build_loader(t2);
                Term::Or(Box::new(t1_d), Box::new(t2_d))
            }
            &Term::Eq(t1, t2) => {
                let t1_d = self.int_build_loader(t1);
                let t2_d = self.int_build_loader(t2);
                Term::Eq(Box::new(t1_d), Box::new(t2_d))
            }
            &Term::Lt(t1, t2) => {
                let t1_d = self.int_build_loader(t1);
                let t2_d = self.int_build_loader(t2);
                Term::Lt(Box::new(t1_d), Box::new(t2_d))
            }
            &Term::Le(t1, t2) => {
                let t1_d = self.int_build_loader(t1);
                let t2_d = self.int_build_loader(t2);
                Term::Le(Box::new(t1_d), Box::new(t2_d))
            }
            &Term::Gt(t1, t2) => {
                let t1_d = self.int_build_loader(t1);
                let t2_d = self.int_build_loader(t2);
                Term::Gt(Box::new(t1_d), Box::new(t2_d))
            }
            &Term::Ge(t1, t2) => {
                let t1_d = self.int_build_loader(t1);
                let t2_d = self.int_build_loader(t2);
                Term::Ge(Box::new(t1_d), Box::new(t2_d))
            }
        }
    }

    fn int_compile(&mut self, t: &Term) {
        match &t {
            &Term::Int(_intval) => {
                None::<bool>.expect("Only compiled terms are supported! -- bug!");
            }
            &Term::Float(_fval) => {
                None::<bool>.expect("Only compiled terms are supported! -- bug!");
            }
            &Term::String(_sval) => {
                None::<bool>.expect("Only compiled terms are supported! -- bug!");
            }
            &Term::Regex(_rval) => {
                None::<bool>.expect("Only compiled terms are supported! -- bug!");
            }
            &Term::Variable(_varname) => {
                None::<bool>.expect("Only compiled terms are supported! -- bug!");
            }
            &Term::LoadedTerm(idx) => {
                self.instructions.push(Instruction::I64Const(*idx));
            }
            &Term::Not(inner) => {
                self.instructions.push(Instruction::GetLocal(
                    self.idx_state.expect("State should be initialized!"),
                ));
                self.int_compile(inner);
                self.instructions.push(Instruction::Call(FUN_RTL_NOT));
            }
            &Term::And(left, right) => {
                self.instructions.push(Instruction::GetLocal(
                    self.idx_state.expect("State should be initialized!"),
                ));
                self.int_compile(left);
                self.int_compile(right);
                self.instructions.push(Instruction::Call(FUN_RTL_AND));
            }
            &Term::Or(left, right) => {
                self.instructions.push(Instruction::GetLocal(
                    self.idx_state.expect("State should be initialized!"),
                ));
                self.int_compile(left);
                self.int_compile(right);
                self.instructions.push(Instruction::Call(FUN_RTL_OR));
            }
            &Term::Eq(left, right) => {
                self.instructions.push(Instruction::GetLocal(
                    self.idx_state.expect("State should be initialized!"),
                ));
                self.int_compile(left);
                self.int_compile(right);
                self.instructions.push(Instruction::Call(FUN_RTL_EQ));
            }
            &Term::Lt(_left, _right) => {
                None::<bool>.expect("The < operator is not yet supported");
            }
            &Term::Le(_left, _right) => {
                None::<bool>.expect("The <= operator is not yet supported");
            }
            &Term::Gt(_left, _right) => {
                None::<bool>.expect("The > operator is not yet supported");
            }
            &Term::Ge(_left, _right) => {
                None::<bool>.expect("The >= oeprator is not yet supported");
            }
        }
    }

    /// Compile an AST to a wasm representation that needs to be instantiated
    /// subsequently. All variables referenced by the AST *must be* defined
    /// by a call to define_var before ```compile``` is called.
    pub fn compile<'b>(&'b mut self, t: &Term) -> CompiledTerm<'b, 'a> {
        let mut locals = Vec::new();
        locals.push(Local::new(4, elements::ValueType::I32));
        self.idx_state = Some(0);
        self.idx_result = Some(1);
        self.idx_mem_buf_ptr = Some(2);
        self.idx_string = Some(3);

        locals.push(Local::new(1, elements::ValueType::I64));
        self.idx_var_result = Some(4);

        self.instructions
            .push(Instruction::I64Const(self.reserved_slots));
        self.instructions.push(Instruction::Call(FUN_MAKE_STATE));
        self.instructions
            .push(Instruction::TeeLocal(self.idx_state.expect("pe")));
        let t1 = self.int_build_loader(t);
        self.instructions.push(Instruction::End);
        let fun_load = self.instructions.clone();

        self.instructions.clear();
        self.instructions
            .push(Instruction::GetLocal(self.idx_state.expect("pe")));
        self.int_compile(&t1);
        self.instructions.push(Instruction::Call(FUN_RTL_GET_BOOL));
        self.instructions.push(Instruction::End);
        let fun_eval = self.instructions.clone();

        self.instructions.clear();
        self.instructions
            .push(Instruction::GetLocal(self.idx_state.expect("pe")));
        self.instructions.push(Instruction::Call(FUN_FREE_STATE));
        self.instructions.push(Instruction::End);
        let fun_cleanup = self.instructions.clone();

        self.instructions.clear();

        let mut module = builder::module();
        let make_state_sig = module.push_signature(
            builder::signature()
                .param()
                .i64()
                .return_type()
                .i32()
                .build_sig(),
        );
        let free_state_sig = module.push_signature(builder::signature().param().i32().build_sig());
        let make_i64_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .param()
                .i64()
                .return_type()
                .i64()
                .build_sig(),
        );
        let make_f64_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .param()
                .f64()
                .return_type()
                .i64()
                .build_sig(),
        );
        let rtl_eq_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .param()
                .i64()
                .return_type()
                .i64()
                .build_sig(),
        );
        let rtl_get_bool_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .return_type()
                .i32()
                .build_sig(),
        );
        let rtl_and_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .param()
                .i64()
                .return_type()
                .i64()
                .build_sig(),
        );
        let rtl_or_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .param()
                .i64()
                .return_type()
                .i64()
                .build_sig(),
        );
        let rtl_not_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .return_type()
                .i64()
                .build_sig(),
        );
        let alloc_string_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .return_type()
                .i32()
                .build_sig(),
        );
        let get_string_buf_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .return_type()
                .i32()
                .build_sig(),
        );
        let make_string_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .param()
                .i32()
                .param()
                .i32()
                .return_type()
                .i64()
                .build_sig(),
        );
        let make_regex_sig = module.push_signature(
            builder::signature()
                .param()
                .i32()
                .param()
                .i64()
                .param()
                .i64()
                .param()
                .i32()
                .param()
                .i32()
                .return_type()
                .i64()
                .build_sig(),
        );

        let module = module
            .function()
            .signature()
            .with_return_type(Some(elements::ValueType::I32))
            .build()
            .body()
            .with_locals(locals)
            .with_instructions(elements::Instructions::new(fun_load))
            .build()
            .build()
            .function()
            .signature()
            .param()
            .i32()
            .return_type()
            .i32()
            .build()
            .body()
            .with_instructions(elements::Instructions::new(fun_eval))
            .build()
            .build()
            .function()
            .signature()
            .param()
            .i32()
            .build()
            .body()
            .with_instructions(elements::Instructions::new(fun_cleanup))
            .build()
            .build()
            .export()
            .field("load")
            .internal()
            .func(13)
            .build()
            .export()
            .field("eval")
            .internal()
            .func(14)
            .build()
            .export()
            .field("cleanup")
            .internal()
            .func(15)
            .build()
            .import() // 0
            .module("wormrtl")
            .field("make_state")
            .external()
            .func(make_state_sig)
            .build()
            .import() // 1
            .module("wormrtl")
            .field("free_state")
            .external()
            .func(free_state_sig)
            .build()
            .import() // 2
            .module("wormrtl")
            .field("make_i64")
            .external()
            .func(make_i64_sig)
            .build()
            .import() // 3
            .module("wormrtl")
            .field("make_f64")
            .external()
            .func(make_f64_sig)
            .build()
            .import() // 4
            .module("wormrtl")
            .field("rtl_eq")
            .external()
            .func(rtl_eq_sig)
            .build()
            .import() // 5
            .module("wormrtl")
            .field("rtl_get_bool")
            .external()
            .func(rtl_get_bool_sig)
            .build()
            .import() // 6
            .module("wormrtl")
            .field("rtl_and")
            .external()
            .func(rtl_and_sig)
            .build()
            .import() // 7
            .module("wormrtl")
            .field("rtl_or")
            .external()
            .func(rtl_or_sig)
            .build()
            .import() // 8
            .module("wormrtl")
            .field("rtl_not")
            .external()
            .func(rtl_not_sig)
            .build()
            .import() // 9
            .module("wormrtl")
            .field("alloc_string")
            .external()
            .func(alloc_string_sig)
            .build()
            .import() // 10
            .module("wormrtl")
            .field("get_string_buf")
            .external()
            .func(get_string_buf_sig)
            .build()
            .import() // 11
            .module("wormrtl")
            .field("make_string")
            .external()
            .func(make_string_sig)
            .build()
            .import() // 12
            .module("wormrtl")
            .field("make_regex")
            .external()
            .func(make_regex_sig)
            .build()
            .import()
            .module("wormrtl")
            .field("memory")
            .external()
            .memory(0, None)
            .build()
            .build();

        let mut v = Vec::new();
        module.serialize(&mut v).unwrap();

        {
            let mut f = std::fs::File::create("/tmp/test.wasm").expect("Test");
            f.write_all(v.as_slice()).expect("write to file");
        }

        let import_object = imports! {
            "env" => {
                "print_str" => func!(print_str),
            },
        };
        let rtl_module_instance = Rc::new(
            self.rtl_module
                .instantiate(&import_object)
                .expect("wormrtl.wasm instance"),
        );

        let formula_module = wasmer_runtime::compile(v.as_slice()).expect("formula.wasm module");
        let mut fm_import_object: wasmer_runtime::ImportObject = imports! {
            "env" => {
                "print_str" => func!(print_str),
            },
            // "wormrtl" => rtl_module_instance,
        };
        fm_import_object.register("wormrtl", rtl_module_instance.clone());
        let fm_instance = formula_module
            .instantiate(&fm_import_object)
            .expect("formula.wasm instance");
        let fm_init: wasmer_runtime::Func<(), (i32)> = fm_instance.func("load").expect("load");
        let fm_init_res = fm_init.call().expect("fm_init");

        CompiledTerm {
            rtl_module_instance,
            fm_import_object,
            formula_module_instance: fm_instance,
            fm_init_res,
            context: self,
        }
    }
}

struct CompilationContext {
    mem: Vec<u8>,
    mem_ptr: usize,
}

impl<'a, 'b> CompiledTerm<'a, 'b> {
    /// Instantiate a compiled term to be used subsequently for
    /// evaluation
    pub fn instantiate(&'b self) -> InstantiatedTerm<'a, 'b> {
        let make_i64_call = self.rtl_module_instance.func("make_i64").expect("make_i64");

        let make_f64_call = self.rtl_module_instance.func("make_f64").expect("make_f64");

        let alloc_string_call = self
            .rtl_module_instance
            .func("alloc_string")
            .expect("alloc_string");

        let get_string_buf_call = self
            .rtl_module_instance
            .func("get_string_buf")
            .expect("get_string_buf");

        let make_string_call = self
            .rtl_module_instance
            .func("make_string")
            .expect("make_string");

        let eval_call = self.formula_module_instance.func("eval").expect("eval");

        let memory = self.rtl_module_instance.context().memory(0);

        InstantiatedTerm {
            make_i64_call,
            make_f64_call,
            alloc_string_call,
            get_string_buf_call,
            make_string_call,
            eval_call,
            memory,
            ct: self,
        }
    }
}

impl<'a, 'b> InstantiatedTerm<'a, 'b> {
    /// Evaluate the formula
    pub fn run(&self) -> bool {
        let res = self.eval_call.call(self.ct.fm_init_res).expect("eval_call");
        res != 0
    }

    /// Retrieve a reference to a variable defined in the context that
    /// created this instance. Subsequent access to the variable should be
    /// O(1).
    pub fn get_variable(&self, var_name: &str) -> Option<Variable> {
        if let Some(var_num) = self.ct.context.variables.get(var_name) {
            Some(Variable {
                idx: *var_num,
                ct: self,
            })
        } else {
            None
        }
    }
}
