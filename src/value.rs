use std::collections::HashMap;
use crate::Condition;

#[derive(Debug, Clone)]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum CompOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
}

#[derive(Debug, Clone)]
pub enum Value {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Usize(usize),
    Bool(bool),
    Str(String),
}

impl Value {
    pub fn to_display_string(&self) -> String {
        match self {
            Value::I8(v)    => v.to_string(),
            Value::U8(v)    => v.to_string(),
            Value::I16(v)   => v.to_string(),
            Value::U16(v)   => v.to_string(),
            Value::I32(v)   => v.to_string(),
            Value::U32(v)   => v.to_string(),
            Value::I64(v)   => v.to_string(),
            Value::U64(v)   => v.to_string(),
            Value::F32(v)   => format!("{:.2}", v),
            Value::F64(v)   => format!("{:.2}", v),
            Value::Usize(v) => v.to_string(),
            Value::Bool(v)  => v.to_string(),
            Value::Str(s)   => s.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Value),
    Var(String),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Format { template: String, args: Vec<Expr> },
}

impl Expr {
    pub fn var(name: impl Into<String>) -> Self { Expr::Var(name.into()) }
    pub fn u8(n: u8)       -> Self { Expr::Lit(Value::U8(n)) }
    pub fn i8(n: i8)       -> Self { Expr::Lit(Value::I8(n)) }
    pub fn u16(n: u16)     -> Self { Expr::Lit(Value::U16(n)) }
    pub fn i16(n: i16)     -> Self { Expr::Lit(Value::I16(n)) }
    pub fn u32(n: u32)     -> Self { Expr::Lit(Value::U32(n)) }
    pub fn i32(n: i32)     -> Self { Expr::Lit(Value::I32(n)) }
    pub fn u64(n: u64)     -> Self { Expr::Lit(Value::U64(n)) }
    pub fn i64(n: i64)     -> Self { Expr::Lit(Value::I64(n)) }
    pub fn f32(n: f32)     -> Self { Expr::Lit(Value::F32(n)) }
    pub fn f64(n: f64)     -> Self { Expr::Lit(Value::F64(n)) }
    pub fn usize(n: usize) -> Self { Expr::Lit(Value::Usize(n)) }
    pub fn str(s: impl Into<String>) -> Self { Expr::Lit(Value::Str(s.into())) }
    pub fn bool(b: bool)   -> Self { Expr::Lit(Value::Bool(b)) }

    pub fn add(a: Expr, b: Expr) -> Self { Expr::Add(Box::new(a), Box::new(b)) }
    pub fn sub(a: Expr, b: Expr) -> Self { Expr::Sub(Box::new(a), Box::new(b)) }
    pub fn mul(a: Expr, b: Expr) -> Self { Expr::Mul(Box::new(a), Box::new(b)) }
    pub fn div(a: Expr, b: Expr) -> Self { Expr::Div(Box::new(a), Box::new(b)) }

    pub fn eq(self, rhs: impl Into<Expr>) -> Condition {
        Condition::Compare(self, CompOp::Eq, rhs.into())
    }

    pub fn ne(self, rhs: impl Into<Expr>) -> Condition {
        Condition::Compare(self, CompOp::Ne, rhs.into())
    }

    pub fn gt(self, rhs: impl Into<Expr>) -> Condition {
        Condition::Compare(self, CompOp::Gt, rhs.into())
    }

    pub fn lt(self, rhs: impl Into<Expr>) -> Condition {
        Condition::Compare(self, CompOp::Lt, rhs.into())
    }

    pub fn gte(self, rhs: impl Into<Expr>) -> Condition {
        Condition::Compare(self, CompOp::Gte, rhs.into())
    }

    pub fn lte(self, rhs: impl Into<Expr>) -> Condition {
        Condition::Compare(self, CompOp::Lte, rhs.into())
    }
}

impl From<Value> for Expr {
    fn from(v: Value) -> Self { Expr::Lit(v) }
}

pub fn resolve_expr(expr: &Expr, vars: &HashMap<String, Value>) -> Option<Value> {
    match expr {
        Expr::Lit(v)          => Some(v.clone()),
        Expr::Var(name)       => vars.get(name).cloned(),
        Expr::Add(a, b)       => compute_math(a, b, vars, &MathOp::Add),
        Expr::Sub(a, b)       => compute_math(a, b, vars, &MathOp::Sub),
        Expr::Mul(a, b)       => compute_math(a, b, vars, &MathOp::Mul),
        Expr::Div(a, b)       => compute_math(a, b, vars, &MathOp::Div),
        Expr::Format { template, args } => {
            let mut result = template.clone();
            for (i, arg) in args.iter().enumerate() {
                if let Some(resolved) = resolve_expr(arg, vars) {
                    result = result.replace(&format!("{{{}}}", i), &resolved.to_display_string());
                }
            }
            Some(Value::Str(result))
        }
    }
}

fn compute_math(a: &Expr, b: &Expr, vars: &HashMap<String, Value>, op: &MathOp) -> Option<Value> {
    let left  = resolve_expr(a, vars)?;
    let right = resolve_expr(b, vars)?;
    apply_op(&left, &right, op)
}

pub fn apply_op(left: &Value, right: &Value, op: &MathOp) -> Option<Value> {
    match (left, right) {
        (Value::I8(l),    Value::I8(r))    => Some(Value::I8(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::U8(l),    Value::U8(r))    => Some(Value::U8(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::I16(l),   Value::I16(r))   => Some(Value::I16(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::U16(l),   Value::U16(r))   => Some(Value::U16(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::I32(l),   Value::I32(r))   => Some(Value::I32(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::U32(l),   Value::U32(r))   => Some(Value::U32(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::I64(l),   Value::I64(r))   => Some(Value::I64(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::U64(l),   Value::U64(r))   => Some(Value::U64(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::Usize(l), Value::Usize(r)) => Some(Value::Usize(match op {
            MathOp::Add => l.saturating_add(*r),
            MathOp::Sub => l.saturating_sub(*r),
            MathOp::Mul => l.saturating_mul(*r),
            MathOp::Div => if *r != 0 { l / r } else { 0 },
        })),
        (Value::F32(l),   Value::F32(r))   => Some(Value::F32(match op {
            MathOp::Add => l + r,
            MathOp::Sub => l - r,
            MathOp::Mul => l * r,
            MathOp::Div => if *r != 0.0 { l / r } else { 0.0 },
        })),
        (Value::F64(l),   Value::F64(r))   => Some(Value::F64(match op {
            MathOp::Add => l + r,
            MathOp::Sub => l - r,
            MathOp::Mul => l * r,
            MathOp::Div => if *r != 0.0 { l / r } else { 0.0 },
        })),
        (Value::Str(l),   Value::Str(r))   => match op {
            MathOp::Add => Some(Value::Str(format!("{}{}", l, r))),
            _ => None,
        },
        _ => None,
    }
}

pub fn compare_operands(left: &Value, right: &Value, op: &CompOp) -> Option<bool> {
    match (left, right) {
        (Value::I8(l),    Value::I8(r))    => Some(compare_ord(l, r, op)),
        (Value::U8(l),    Value::U8(r))    => Some(compare_ord(l, r, op)),
        (Value::I16(l),   Value::I16(r))   => Some(compare_ord(l, r, op)),
        (Value::U16(l),   Value::U16(r))   => Some(compare_ord(l, r, op)),
        (Value::I32(l),   Value::I32(r))   => Some(compare_ord(l, r, op)),
        (Value::U32(l),   Value::U32(r))   => Some(compare_ord(l, r, op)),
        (Value::I64(l),   Value::I64(r))   => Some(compare_ord(l, r, op)),
        (Value::U64(l),   Value::U64(r))   => Some(compare_ord(l, r, op)),
        (Value::Usize(l), Value::Usize(r)) => Some(compare_ord(l, r, op)),
        (Value::F32(l),   Value::F32(r))   => Some(compare_ord(l, r, op)),
        (Value::F64(l),   Value::F64(r))   => Some(compare_ord(l, r, op)),
        (Value::Bool(l),  Value::Bool(r))  => Some(match op {
            CompOp::Eq => l == r,
            CompOp::Ne => l != r,
            _ => false,
        }),
        _ => None,
    }
}

fn compare_ord<T: PartialOrd + PartialEq>(l: &T, r: &T, op: &CompOp) -> bool {
    match op {
        CompOp::Eq  => l == r,
        CompOp::Ne  => l != r,
        CompOp::Gt  => l > r,
        CompOp::Lt  => l < r,
        CompOp::Gte => l >= r,
        CompOp::Lte => l <= r,
    }
}

impl From<u8>    for Value { fn from(v: u8)    -> Self { Value::U8(v) } }
impl From<i8>    for Value { fn from(v: i8)    -> Self { Value::I8(v) } }
impl From<u16>   for Value { fn from(v: u16)   -> Self { Value::U16(v) } }
impl From<i16>   for Value { fn from(v: i16)   -> Self { Value::I16(v) } }
impl From<u32>   for Value { fn from(v: u32)   -> Self { Value::U32(v) } }
impl From<i32>   for Value { fn from(v: i32)   -> Self { Value::I32(v) } }
impl From<u64>   for Value { fn from(v: u64)   -> Self { Value::U64(v) } }
impl From<i64>   for Value { fn from(v: i64)   -> Self { Value::I64(v) } }
impl From<f32>   for Value { fn from(v: f32)   -> Self { Value::F32(v) } }
impl From<f64>   for Value { fn from(v: f64)   -> Self { Value::F64(v) } }
impl From<usize> for Value { fn from(v: usize) -> Self { Value::Usize(v) } }
impl From<bool>  for Value { fn from(v: bool)  -> Self { Value::Bool(v) } }
impl From<String> for Value { fn from(v: String) -> Self { Value::Str(v) } }
impl From<&str>  for Value { fn from(v: &str)  -> Self { Value::Str(v.to_string()) } }

impl From<u8>    for Expr { fn from(v: u8)    -> Self { Expr::from(Value::U8(v)) } }
impl From<i8>    for Expr { fn from(v: i8)    -> Self { Expr::from(Value::I8(v)) } }
impl From<u16>   for Expr { fn from(v: u16)   -> Self { Expr::from(Value::U16(v)) } }
impl From<i16>   for Expr { fn from(v: i16)   -> Self { Expr::from(Value::I16(v)) } }
impl From<u32>   for Expr { fn from(v: u32)   -> Self { Expr::from(Value::U32(v)) } }
impl From<i32>   for Expr { fn from(v: i32)   -> Self { Expr::from(Value::I32(v)) } }
impl From<u64>   for Expr { fn from(v: u64)   -> Self { Expr::from(Value::U64(v)) } }
impl From<i64>   for Expr { fn from(v: i64)   -> Self { Expr::from(Value::I64(v)) } }
impl From<f32>   for Expr { fn from(v: f32)   -> Self { Expr::from(Value::F32(v)) } }
impl From<f64>   for Expr { fn from(v: f64)   -> Self { Expr::from(Value::F64(v)) } }
impl From<usize> for Expr { fn from(v: usize) -> Self { Expr::from(Value::Usize(v)) } }
impl From<bool>  for Expr { fn from(v: bool)  -> Self { Expr::from(Value::Bool(v)) } }
impl From<String> for Expr { fn from(v: String) -> Self { Expr::from(Value::Str(v)) } }
impl From<&str>  for Expr { fn from(v: &str)  -> Self { Expr::from(Value::Str(v.to_string())) } }