use std::collections::HashMap;

//==================================================================================
//Requirements for Compare and Math Operations on Values
//==================================================================================

pub trait Numeric: PartialOrd + PartialEq + std::fmt::Debug {}
impl Numeric for i8 {}
impl Numeric for u8 {}
impl Numeric for i16 {}
impl Numeric for u16 {}
impl Numeric for i32 {}
impl Numeric for u32 {}
impl Numeric for i64 {}
impl Numeric for u64 {}
impl Numeric for f32 {}
impl Numeric for f64 {}
impl Numeric for usize {}

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
    Var(String),                  // Variable reference
    Add(Box<Value>, Box<Value>),  // left + right
    Sub(Box<Value>, Box<Value>),  // left - right
    Mul(Box<Value>, Box<Value>),  // left * right
    Div(Box<Value>, Box<Value>),  // left / right    
}

impl Value {
    pub fn var(name: impl Into<String>) -> Self {
        Value::Var(name.into())
    }

    pub fn add(a: Value, b: Value) -> Self {
        Value::Add(Box::new(a), Box::new(b))
    }

    pub fn sub(a: Value, b: Value) -> Self {
        Value::Sub(Box::new(a), Box::new(b))
    }

    pub fn mul(a: Value, b: Value) -> Self {
        Value::Mul(Box::new(a), Box::new(b))
    }

    pub fn div(a: Value, b: Value) -> Self {
        Value::Div(Box::new(a), Box::new(b))
    }
}

#[derive(Debug, Clone)]
pub enum MathOperator {
    Add,
    Sub,
    Mul,
    Div,
}
pub fn resolve_value(value: &Value, vars: &HashMap<String,Value>) -> Option<Value> {
    match value {
        Value::I8(_) | Value::U8(_) | Value::I16(_) | Value::U16(_) | Value::I32(_) | Value::U32(_) | Value::I64(_) | Value::U64(_) | Value::F32(_) | Value::F64(_) | Value::Usize(_) | Value::Bool(_) => {
            Some(value.clone())
        }
        Value::Var(name) => vars.get(name).cloned(),
        Value::Add(a, b) => compute_math(a, b, vars, &MathOperator::Add),
        Value::Sub(a, b) => compute_math(a, b, vars, &MathOperator::Sub),
        Value::Mul(a, b) => compute_math(a, b, vars, &MathOperator::Mul),
        Value::Div(a, b) => compute_math(a, b, vars, &MathOperator::Div),
    }
}

fn compute_math(
    a: &Value,
    b: &Value,
    vars: &HashMap<String, Value>,
    op: &MathOperator,    
) -> Option<Value> {
    let left = resolve_value(a, vars)?;
    let right = resolve_value(b, vars)?;
    apply_op(&left, op, &right)
}

pub fn apply_op(left: &Value, op: &MathOperator, right: &Value) -> Option<Value> {
    match (left, right) {
        (Value::U32(l), Value::U32(r)) => Some(match op {
            MathOperator::Add => Value::U32(l.saturating_add(*r)),
            MathOperator::Sub => Value::U32(l.saturating_sub(*r)),
            MathOperator::Mul => Value::U32(l.saturating_mul(*r)),
            MathOperator::Div => if *r != 0 {
                Value::U32(l / r)
            } else {
                return None; // Division by zero
            },
        }),
        (Value::I32(l), Value::I32(r)) => Some(match op {
            MathOperator::Add => Value::I32(l.saturating_add(*r)),
            MathOperator::Sub => Value::I32(l.saturating_sub(*r)),
            MathOperator::Mul => Value::I32(l.saturating_mul(*r)),
            MathOperator::Div => if *r != 0 {
                Value::I32(l / r)
            } else {
                return None; // Division by zero
            },
        }),
        (Value::I64(l), Value::I64(r)) => Some(match op {
            MathOperator::Add => Value::I64(l.saturating_add(*r)),
            MathOperator::Sub => Value::I64(l.saturating_sub(*r)),
            MathOperator::Mul => Value::I64(l.saturating_mul(*r)),
            MathOperator::Div => if *r != 0 {
                Value::I64(l / r)
            } else {
                return None; // Division by zero
            },
        }),
        (Value::U64(l), Value::U64(r)) => Some(match op {
            MathOperator::Add => Value::U64(l.saturating_add(*r)),
            MathOperator::Sub => Value::U64(l.saturating_sub(*r)),
            MathOperator::Mul => Value::U64(l.saturating_mul(*r)),
            MathOperator::Div => if *r != 0 {
                Value::U64(l / r)
            } else {
                return None; // Division by zero
            },
        }),
        (Value::F32(l), Value::F32(r)) => Some(match op {
            MathOperator::Add => Value::F32(l + r),
            MathOperator::Sub => Value::F32(l - r),
            MathOperator::Mul => Value::F32(l * r),
            MathOperator::Div => if *r != 0.0 {
                Value::F32(l / r)
            } else {
                return None; // Division by zero
            },
        }),
        (Value::F64(l), Value::F64(r)) => Some(match op {
            MathOperator::Add => Value::F64(l + r),
            MathOperator::Sub => Value::F64(l - r),
            MathOperator::Mul => Value::F64(l * r),
            MathOperator::Div => if *r != 0.0 {
                Value::F64(l / r)
            } else {
                return None; // Division by zero
            },
        }),
        (Value::Usize(l), Value::Usize(r)) => Some(match op {
            MathOperator::Add => Value::Usize(l.saturating_add(*r)),
            MathOperator::Sub => Value::Usize(l.saturating_sub(*r)),
            MathOperator::Mul => Value::Usize(l.saturating_mul(*r)),
            MathOperator::Div => if *r != 0 {
                Value::Usize(l / r)
            } else {
                return None; // Division by zero
            },
        }),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
}

fn compare_numeric<T: Numeric>(l: &T, op: &ComparisonOperator, r: &T) -> bool {
    match op {
        ComparisonOperator::Equal => l == r,
        ComparisonOperator::NotEqual => l != r,
        ComparisonOperator::Less => l < r,
        ComparisonOperator::Greater => l > r,
        ComparisonOperator::LessOrEqual => l <= r,
        ComparisonOperator::GreaterOrEqual => l >= r,
    }
}

pub fn compare_operands(left: &Value, op: &ComparisonOperator, right: &Value) -> Option<bool> {
    match (left, right) {
        (Value::I8(l), Value::I8(r)) => Some(compare_numeric(l, op, r)),
        (Value::U8(l), Value::U8(r)) => Some(compare_numeric(l, op, r)),
        (Value::I16(l), Value::I16(r)) => Some(compare_numeric(l, op, r)),
        (Value::U16(l), Value::U16(r)) => Some(compare_numeric(l, op, r)),
        (Value::I32(l), Value::I32(r)) => Some(compare_numeric(l, op, r)),
        (Value::U32(l), Value::U32(r)) => Some(compare_numeric(l, op, r)),
        (Value::I64(l), Value::I64(r)) => Some(compare_numeric(l, op, r)),
        (Value::U64(l), Value::U64(r)) => Some(compare_numeric(l, op, r)),
        (Value::F32(l), Value::F32(r)) => Some(compare_numeric(l, op, r)),
        (Value::F64(l), Value::F64(r)) => Some(compare_numeric(l, op, r)),
        (Value::Usize(l), Value::Usize(r)) => Some(compare_numeric(l, op, r)),
        (Value::Bool(l), Value::Bool(r)) => Some(match op {
            ComparisonOperator::Equal => l == r,
            ComparisonOperator::NotEqual => l != r,
            _ => false, // Only equality makes sense for bools
        }),
        _ => None, // Type mismatch
    }
}

//==================================================================================
//End of edits 3/12/2026
//==================================================================================