use std::collections::HashMap;
use std::ffi::CStr;

use cranelift::frontend::FunctionBuilder;
use cranelift::prelude::{types, InstBuilder, Value};
use cranelift_jit::JITBuilder;

use crate::frontend::Arg;
use crate::hashmap;
use crate::jit::SValue;
use crate::{
    frontend::{Declaration, Function},
    validator::ExprType,
};

fn decl(name: &str, params: Vec<(&str, ExprType)>, returns: Vec<(&str, ExprType)>) -> Declaration {
    Declaration::Function(Function {
        name: name.to_string(),
        params: params
            .into_iter()
            .map(|(name, expr)| Arg {
                name: name.to_string(),
                expr_type: Some(expr),
            })
            .collect(),
        returns: returns
            .into_iter()
            .map(|(name, expr)| Arg {
                name: name.to_string(),
                expr_type: Some(expr),
            })
            .collect(),
        body: vec![],
        extern_func: true,
    })
}

//Reference: https://www.gnu.org/software/libc/manual/html_node/Mathematics.html
//https://docs.rs/libc/0.2.101/libc/
//should this include bessel functions? It seems like they would pollute the name space.

//couldn't get to work (STATUS_ACCESS_VIOLATION):
// "asinh", "acosh", "atanh", "erf", "erfc", "lgamma", "gamma", "tgamma", "exp2", "exp10", "log2"
const STD_1ARG_FF: [&str; 20] = [
    "sin", // libc
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "exp",
    "log",
    "log10",
    "sqrt",
    "sinh",
    "cosh",
    "exp10",
    "tanh",
    "f64.ceil", // built in std
    "f64.floor",
    "f64.trunc",
    "f64.fract",
    "f64.abs",
    "f64.round",
];
const STD_1ARG_IF: [&str; 1] = [
    "i64.f64", // built in std
];
const STD_1ARG_FI: [&str; 1] = [
    "f64.i64", // built in std
];

//couldn't get to work (STATUS_ACCESS_VIOLATION):
// "hypot", "expm1", "log1p"
const STD_2ARG_FF: [&str; 4] = [
    "atan2", "pow", // libc
    "f64.min", "f64.max", // built in std
];
const STD_2ARG_II: [&str; 2] = [
    "i64.min", "i64.max", // built in std
];

extern "C" fn f64_print(x: f64) {
    print!("{}", x);
}

extern "C" fn i64_print(x: i64) {
    print!("{}", x);
}

extern "C" fn bool_print(x: bool) {
    print!("{}", x);
}

extern "C" fn str_print(s: *const i8) {
    unsafe {
        print!("{}", CStr::from_ptr(s).to_str().unwrap());
    }
}

extern "C" fn f64_println(x: f64) {
    println!("{}", x);
}

extern "C" fn i64_println(x: i64) {
    println!("{}", x);
}

extern "C" fn bool_println(x: bool) {
    println!("{}", x);
}

extern "C" fn str_println(s: *const i8) {
    unsafe {
        println!("{}", CStr::from_ptr(s).to_str().unwrap());
    }
}

extern "C" fn f64_assert_eq(x: f64, y: f64) {
    assert_eq!(x, y);
}

extern "C" fn i64_assert_eq(x: i64, y: i64) {
    assert_eq!(x, y);
}

extern "C" fn bool_assert_eq(x: bool, y: bool) {
    assert_eq!(x, y);
}

extern "C" fn str_assert_eq(s1: *const i8, s2: *const i8) {
    unsafe {
        let s1 = CStr::from_ptr(s1).to_str().unwrap();
        let s2 = CStr::from_ptr(s2).to_str().unwrap();
        assert_eq!(s1, s2);
    }
}

pub fn append_std_symbols(jit_builder: &mut JITBuilder) {
    jit_builder.symbols([
        ("f64.print", f64_print as *const u8),
        ("i64.print", i64_print as *const u8),
        ("bool.print", bool_print as *const u8),
        ("&.print", str_print as *const u8), //TODO setup actual str type
        ("f64.println", f64_println as *const u8),
        ("i64.println", i64_println as *const u8),
        ("bool.println", bool_println as *const u8),
        ("&.println", str_println as *const u8), //TODO setup actual str type
        ("f64.assert_eq", f64_assert_eq as *const u8),
        ("i64.assert_eq", i64_assert_eq as *const u8),
        ("bool.assert_eq", bool_assert_eq as *const u8),
        ("&.assert_eq", str_assert_eq as *const u8), //TODO setup actual str type
    ]);
}

pub fn append_std_funcs(mut prog: Vec<Declaration>) -> Vec<Declaration> {
    for n in STD_1ARG_FF {
        prog.push(decl(
            n,
            vec![("x", ExprType::F64)],
            vec![("y", ExprType::F64)],
        ));
    }
    for n in STD_1ARG_FI {
        prog.push(decl(
            n,
            vec![("x", ExprType::F64)],
            vec![("y", ExprType::I64)],
        ));
    }
    for n in STD_1ARG_IF {
        prog.push(decl(
            n,
            vec![("x", ExprType::I64)],
            vec![("y", ExprType::F64)],
        ));
    }
    for n in STD_2ARG_FF {
        prog.push(decl(
            n,
            vec![("x", ExprType::F64), ("y", ExprType::F64)],
            vec![("z", ExprType::F64)],
        ));
    }
    for n in STD_2ARG_II {
        prog.push(decl(
            n,
            vec![("x", ExprType::I64), ("y", ExprType::I64)],
            vec![("z", ExprType::I64)],
        ));
    }
    prog.push(decl("f64.print", vec![("x", ExprType::F64)], vec![]));
    prog.push(decl("i64.print", vec![("x", ExprType::I64)], vec![]));
    prog.push(decl("bool.print", vec![("x", ExprType::Bool)], vec![]));
    prog.push(decl("&.print", vec![("x", ExprType::Address)], vec![]));
    prog.push(decl("f64.println", vec![("x", ExprType::F64)], vec![]));
    prog.push(decl("i64.println", vec![("x", ExprType::I64)], vec![]));
    prog.push(decl("bool.println", vec![("x", ExprType::Bool)], vec![]));
    prog.push(decl("&.println", vec![("x", ExprType::Address)], vec![]));
    prog.push(decl(
        "f64.assert_eq",
        vec![("x", ExprType::F64), ("y", ExprType::F64)],
        vec![],
    ));
    prog.push(decl(
        "i64.assert_eq",
        vec![("x", ExprType::I64), ("y", ExprType::I64)],
        vec![],
    ));
    prog.push(decl(
        "bool.assert_eq",
        vec![("x", ExprType::Bool), ("y", ExprType::Bool)],
        vec![],
    ));
    prog.push(decl(
        "&.assert_eq",
        vec![("x", ExprType::Address), ("y", ExprType::Address)],
        vec![],
    ));

    //prog.push(decl(
    //    "bytes",
    //    vec![("size", ExprType::I64)],
    //    vec![("mem", ExprType::Address)],
    //));
    prog
}

pub(crate) fn translate_std(
    _ptr_ty: cranelift::prelude::Type,
    builder: &mut FunctionBuilder,
    name: &str,
    args: &[Value],
) -> anyhow::Result<Option<SValue>> {
    match name {
        "f64.trunc" => Ok(Some(SValue::F64(builder.ins().trunc(args[0])))),
        "f64.floor" => Ok(Some(SValue::F64(builder.ins().floor(args[0])))),
        "f64.ceil" => Ok(Some(SValue::F64(builder.ins().ceil(args[0])))),
        "f64.fract" => {
            let v_int = builder.ins().trunc(args[0]);
            let v = builder.ins().fsub(args[0], v_int);
            Ok(Some(SValue::F64(v)))
        }
        "f64.abs" => Ok(Some(SValue::F64(builder.ins().fabs(args[0])))),
        "f64.round" => Ok(Some(SValue::F64(builder.ins().nearest(args[0])))),
        "f64.i64" => Ok(Some(SValue::I64(
            builder.ins().fcvt_to_sint(types::I64, args[0]),
        ))),
        "i64.f64" => Ok(Some(SValue::F64(
            builder.ins().fcvt_from_sint(types::F64, args[0]),
        ))),
        "f64.min" => Ok(Some(SValue::F64(builder.ins().fmin(args[0], args[1])))),
        "f64.max" => Ok(Some(SValue::F64(builder.ins().fmax(args[0], args[1])))),
        "i64.min" => Ok(Some(SValue::I64(builder.ins().imin(args[0], args[1])))),
        "i64.max" => Ok(Some(SValue::I64(builder.ins().imax(args[0], args[1])))),
        _ => Ok(None),
    }
}

pub fn get_constants() -> HashMap<String, f64> {
    hashmap!(
        "E".into() => std::f64::consts::E,
        "FRAC_1_PI".into() => std::f64::consts::FRAC_1_PI,
        "FRAC_1_SQRT_2".into() => std::f64::consts::FRAC_1_SQRT_2,
        "FRAC_2_SQRT_PI".into() => std::f64::consts::FRAC_2_SQRT_PI,
        "FRAC_PI_2".into() => std::f64::consts::FRAC_PI_2,
        "FRAC_PI_3".into() => std::f64::consts::FRAC_PI_3,
        "FRAC_PI_4".into() => std::f64::consts::FRAC_PI_4,
        "FRAC_PI_6".into() => std::f64::consts::FRAC_PI_6,
        "FRAC_PI_8".into() => std::f64::consts::FRAC_PI_8,
        "LN_2".into() => std::f64::consts::LN_2,
        "LN_10".into() => std::f64::consts::LN_10,
        "LOG2_10".into() => std::f64::consts::LOG2_10,
        "LOG2_E".into() => std::f64::consts::LOG2_E,
        "LOG10_2".into() => std::f64::consts::LOG10_2,
        "LOG10_E".into() => std::f64::consts::LOG10_E,
        "PI".into() => std::f64::consts::PI,
        "SQRT_2".into() => std::f64::consts::SQRT_2,
        "TAU".into() => std::f64::consts::TAU
    )
}
