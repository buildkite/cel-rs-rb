use cel::{Context as CelContext, ExecutionError as CelExecutionError, FunctionContext, ParseErrors, Program as CelProgram, ResolveResult, Value as CelValue};
use magnus::block::Proc;
use magnus::prelude::*;
use magnus::{function, method, Error, IntoValue, RHash, Ruby, TryConvert, Value};
use rb_sys::{rb_thread_call_with_gvl, rb_thread_call_without_gvl};
use std::collections::HashMap;
use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

mod errors {
    use magnus::prelude::*;
    use magnus::{Error, ExceptionClass, RModule, Ruby};
    use std::cell::RefCell;

    thread_local! {
        static PARSE: RefCell<Option<ExceptionClass>> = const { RefCell::new(None) };
        static EXECUTION: RefCell<Option<ExceptionClass>> = const { RefCell::new(None) };
        static TYPE: RefCell<Option<ExceptionClass>> = const { RefCell::new(None) };
    }

    pub fn define(ruby: &Ruby, module: &RModule) -> Result<(), Error> {
        let standard = ruby.exception_standard_error();
        let base = module.define_error("Error", standard)?;
        let parse = module.define_error("ParseError", base)?;
        let execution = module.define_error("ExecutionError", base)?;
        let ty = module.define_error("TypeError", base)?;
        PARSE.with(|slot| *slot.borrow_mut() = Some(parse));
        EXECUTION.with(|slot| *slot.borrow_mut() = Some(execution));
        TYPE.with(|slot| *slot.borrow_mut() = Some(ty));
        Ok(())
    }

    fn fallback(msg: String) -> Error {
        let ruby = Ruby::get().expect("ruby runtime");
        Error::new(ruby.exception_runtime_error(), msg)
    }

    pub fn parse(msg: impl Into<String>) -> Error {
        let msg = msg.into();
        PARSE.with(|slot| {
            slot.borrow()
                .map(|exc| Error::new(exc, msg.clone()))
                .unwrap_or_else(|| fallback(msg))
        })
    }

    pub fn execution(msg: impl Into<String>) -> Error {
        let msg = msg.into();
        EXECUTION.with(|slot| {
            slot.borrow()
                .map(|exc| Error::new(exc, msg.clone()))
                .unwrap_or_else(|| fallback(msg))
        })
    }

    pub fn ty(msg: impl Into<String>) -> Error {
        let msg = msg.into();
        TYPE.with(|slot| {
            slot.borrow()
                .map(|exc| Error::new(exc, msg.clone()))
                .unwrap_or_else(|| fallback(msg))
        })
    }
}

fn without_gvl<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
    T: Send,
{
    struct State<F, T>
    where
        F: FnOnce() -> T,
        T: Send,
    {
        f: Option<F>,
        result: Option<T>,
    }

    unsafe extern "C" fn call<F, T>(ptr: *mut c_void) -> *mut c_void
    where
        F: FnOnce() -> T,
        T: Send,
    {
        let state = &mut *(ptr as *mut State<F, T>);
        let fun = state.f.take().expect("closure missing");
        state.result = Some(fun());
        std::ptr::null_mut()
    }

    let mut state = State {
        f: Some(f),
        result: None,
    };

    unsafe {
        rb_thread_call_without_gvl(
            Some(call::<F, T>),
            &mut state as *mut _ as *mut c_void,
            None,
            std::ptr::null_mut(),
        );
    }

    state.result.expect("result missing")
}

fn with_gvl<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    struct State<F, T>
    where
        F: FnOnce() -> T,
    {
        f: Option<F>,
        result: Option<T>,
    }

    unsafe extern "C" fn call<F, T>(ptr: *mut c_void) -> *mut c_void
    where
        F: FnOnce() -> T,
    {
        let state = &mut *(ptr as *mut State<F, T>);
        let fun = state.f.take().expect("closure missing");
        state.result = Some(fun());
        std::ptr::null_mut()
    }

    let mut state = State {
        f: Some(f),
        result: None,
    };
    unsafe {
        rb_thread_call_with_gvl(Some(call::<F, T>), &mut state as *mut _ as *mut c_void);
    }
    state.result.expect("result missing")
}

#[derive(Clone)]
struct CallbackFunction {
    proc: Proc,
}
unsafe impl Send for CallbackFunction {}
unsafe impl Sync for CallbackFunction {}

#[derive(Clone)]
struct FunctionRegistration {
    name: String,
    callback: Arc<CallbackFunction>,
}

#[derive(Default)]
#[magnus::wrap(class = "CEL::Context", free_immediately, size)]
struct ContextWrap {
    use_empty: bool,
    variables: Mutex<HashMap<String, CelValue>>,
    functions: Mutex<Vec<FunctionRegistration>>,
}

#[magnus::wrap(class = "CEL::Program", free_immediately, size)]
struct ProgramWrap {
    inner: CelProgram,
}

fn ruby_to_cel_value(value: Value) -> Result<CelValue, Error> {
    let ruby = Ruby::get().expect("ruby runtime");

    if value.is_nil() {
        return Ok(CelValue::Null);
    }
    if value.is_kind_of(ruby.class_true_class()) || value.is_kind_of(ruby.class_false_class()) {
        return Ok(CelValue::Bool(<bool as TryConvert>::try_convert(value)?));
    }
    if value.is_kind_of(ruby.class_integer()) {
        return Ok(CelValue::Int(<i64 as TryConvert>::try_convert(value)?));
    }
    if value.is_kind_of(ruby.class_float()) {
        return Ok(CelValue::Float(<f64 as TryConvert>::try_convert(value)?));
    }
    if value.is_kind_of(ruby.class_string()) {
        return Ok(CelValue::String(Arc::new(<String as TryConvert>::try_convert(value)?)));
    }
    if value.is_kind_of(ruby.class_symbol()) {
        let sym = <magnus::Symbol as TryConvert>::try_convert(value)?;
        return Ok(CelValue::String(Arc::new(sym.name()?.to_string())));
    }
    if value.is_kind_of(ruby.class_array()) {
        let array = <magnus::RArray as TryConvert>::try_convert(value)?;
        let mut out = Vec::with_capacity(array.len());
        for element in array.each() {
            out.push(ruby_to_cel_value(element?)?);
        }
        return Ok(CelValue::List(Arc::new(out)));
    }
    if value.is_kind_of(ruby.class_hash()) {
        let hash = <RHash as TryConvert>::try_convert(value)?;
        let mut out = HashMap::new();
        hash.foreach(|k: Value, v: Value| {
            let key = if let Ok(s) = <String as TryConvert>::try_convert(k) {
                cel::objects::Key::from(s)
            } else if let Ok(sym) = <magnus::Symbol as TryConvert>::try_convert(k) {
                cel::objects::Key::from(sym.name()?.to_string())
            } else if let Ok(i) = <i64 as TryConvert>::try_convert(k) {
                cel::objects::Key::from(i)
            } else if let Ok(b) = <bool as TryConvert>::try_convert(k) {
                cel::objects::Key::from(b)
            } else {
                return Err(errors::ty("Hash keys must be String/Symbol/Integer/Boolean"));
            };
            out.insert(key, ruby_to_cel_value(v)?);
            Ok(magnus::r_hash::ForEach::Continue)
        })?;
        return Ok(CelValue::Map(cel::objects::Map { map: Arc::new(out) }));
    }

    Err(errors::ty("Unsupported Ruby type"))
}

fn cel_to_ruby(ruby: &Ruby, value: &CelValue) -> Result<Value, Error> {
    Ok(match value {
        CelValue::Int(v) => (*v).into_value_with(ruby),
        CelValue::UInt(v) => (*v).into_value_with(ruby),
        CelValue::Float(v) => (*v).into_value_with(ruby),
        CelValue::String(v) => v.to_string().into_value_with(ruby),
        CelValue::Bool(v) => (*v).into_value_with(ruby),
        CelValue::Null => ruby.qnil().as_value(),
        CelValue::List(v) => {
            let ary = ruby.ary_new();
            for element in v.iter() {
                ary.push(cel_to_ruby(ruby, element)?)?;
            }
            ary.into_value_with(ruby)
        }
        CelValue::Map(v) => {
            let hash = ruby.hash_new();
            for (k, val) in v.map.iter() {
                let key: Value = match k {
                    cel::objects::Key::Int(i) => (*i).into_value_with(ruby),
                    cel::objects::Key::Uint(u) => (*u).into_value_with(ruby),
                    cel::objects::Key::Bool(b) => (*b).into_value_with(ruby),
                    cel::objects::Key::String(s) => s.to_string().into_value_with(ruby),
                };
                hash.aset(key, cel_to_ruby(ruby, val)?)?;
            }
            hash.into_value_with(ruby)
        }
        _ => return Err(errors::ty(format!("Unsupported CEL value variant: {value:?}"))),
    })
}

impl ContextWrap {
    fn new(empty: bool) -> Self {
        Self {
            use_empty: empty,
            variables: Mutex::new(HashMap::new()),
            functions: Mutex::new(Vec::new()),
        }
    }

    fn add_variable(&self, name: String, value: Value) -> Result<(), Error> {
        self.variables
            .lock()
            .unwrap()
            .insert(name, ruby_to_cel_value(value)?);
        Ok(())
    }

    fn add_function(&self, name: String, proc: Proc) {
        self.functions.lock().unwrap().push(FunctionRegistration {
            name,
            callback: Arc::new(CallbackFunction { proc }),
        });
    }

    fn build_context(&self) -> CelContext<'static> {
        let mut ctx = if self.use_empty {
            CelContext::empty()
        } else {
            CelContext::default()
        };

        for (name, value) in self.variables.lock().unwrap().iter() {
            ctx.add_variable_from_value(name.as_str(), value.clone());
        }

        for registration in self.functions.lock().unwrap().iter() {
            let callback = registration.callback.clone();
            let function_name = registration.name.clone();
            ctx.add_function(
                &function_name,
                move |ftx: &FunctionContext,
                      cel::extractors::Arguments(args): cel::extractors::Arguments|
                      -> ResolveResult {
                    let callback = callback.clone();
                    let this = ftx.this.clone();
                    let args = args.clone();

                    with_gvl(|| {
                        let ruby = Ruby::get().expect("ruby runtime");
                        let mut ruby_args = Vec::new();

                        if let Some(target) = this {
                            ruby_args.push(
                                cel_to_ruby(&ruby, &target).map_err(|e| {
                                    CelExecutionError::function_error(ftx.name, e.to_string())
                                })?,
                            );
                        }

                        for arg in args.iter() {
                            ruby_args.push(cel_to_ruby(&ruby, arg).map_err(|e| {
                                CelExecutionError::function_error(ftx.name, e.to_string())
                            })?);
                        }

                        let proc_result = callback.proc.call(ruby_args.as_slice()).map_err(|e| {
                            CelExecutionError::function_error(
                                ftx.name,
                                format!("Ruby callback error: {e}"),
                            )
                        })?;

                        ruby_to_cel_value(proc_result)
                            .map_err(|e| CelExecutionError::function_error(ftx.name, e.to_string()))
                    })
                },
            );
        }

        ctx
    }
}

impl ProgramWrap {
    fn compile(source: String) -> Result<Self, Error> {
        CelProgram::compile(&source)
            .map(|inner| Self { inner })
            .map_err(|e: ParseErrors| errors::parse(e.to_string()))
    }

    fn execute(&self) -> Result<Value, Error> {
        self.execute_with_context_internal(&CelContext::default())
    }

    fn execute_with_context(&self, context: &ContextWrap) -> Result<Value, Error> {
        self.execute_with_context_internal(&context.build_context())
    }

    fn execute_with_context_internal(&self, ctx: &CelContext<'_>) -> Result<Value, Error> {
        let run = || self.inner.execute(ctx);
        let result = panic::catch_unwind(AssertUnwindSafe(|| without_gvl(run)))
            .map_err(|_| errors::execution("CEL execution panicked"))?;

        let ruby = Ruby::get().expect("ruby runtime");
        result
            .map_err(|e| errors::execution(e.to_string()))
            .and_then(|value| cel_to_ruby(&ruby, &value))
    }

    fn references(&self) -> Result<RHash, Error> {
        let ruby = Ruby::get().expect("ruby runtime");
        let refs = self.inner.references();

        let vars = ruby.ary_new();
        for var in refs.variables() {
            vars.push(var)?;
        }

        let funcs = ruby.ary_new();
        for func in refs.functions() {
            funcs.push(func)?;
        }

        let out = ruby.hash_new();
        out.aset("variables", vars)?;
        out.aset("functions", funcs)?;
        Ok(out)
    }

    fn expression(&self) -> String {
        format!("{:?}", self.inner.expression())
    }
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("CEL")?;
    errors::define(ruby, &module)?;

    let context_class = module.define_class("Context", ruby.class_object())?;
    context_class.define_singleton_method("new", function!(ContextWrap::new, 1))?;
    context_class.define_method("add_variable", method!(ContextWrap::add_variable, 2))?;
    context_class.define_method("[]=", method!(ContextWrap::add_variable, 2))?;
    context_class.define_method("add_function", method!(ContextWrap::add_function, 2))?;

    let program_class = module.define_class("Program", ruby.class_object())?;
    program_class.define_singleton_method("compile", function!(ProgramWrap::compile, 1))?;
    program_class.define_method("execute", method!(ProgramWrap::execute, 0))?;
    program_class.define_method("execute_with_context", method!(ProgramWrap::execute_with_context, 1))?;
    program_class.define_method("references", method!(ProgramWrap::references, 0))?;
    program_class.define_method("expression", method!(ProgramWrap::expression, 0))?;

    module.define_singleton_method("compile", function!(ProgramWrap::compile, 1))?;

    Ok(())
}
