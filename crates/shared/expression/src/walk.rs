//! One object at a time (docs/adr/0100): the expression's tree walked with
//! the language's rules (`scalar`), the same rules the column engine applies
//! a batch at a time. A symbol preview or a single object reads its value
//! here without setting up a batch; text is borrowed from the object and the
//! source where the result is part of them, and made only for text the
//! expression makes, as before the column engine.

use std::borrow::Cow;

use crate::functions;
use crate::library::Var;
use crate::parser::{BinOp, Node};
use crate::scalar::{self, R, Scratch, V};
use crate::value::Value;
use crate::{Geometry, Measured, Scope};

/// What JavaScript would have thrown at: the whole expression is empty.
pub struct Thrown;

fn opt(x: Option<f64>) -> Value<'static> {
    x.map_or(Value::Null, Value::Num)
}

fn borrowed(t: &str) -> Value<'_> {
    Value::Text(Cow::Borrowed(t))
}

fn variable(v: Var, s: &dyn Scope) -> Value<'_> {
    let m = || -> Measured { s.measured() };
    match v {
        Var::Area => opt(m().area),
        Var::Length => opt(m().length),
        Var::Y => opt(m().anchor.map(|a| a.0)),
        Var::X => opt(m().anchor.map(|a| a.1)),
        Var::Vertices => opt(s.vertices()),
        Var::Kind => borrowed(s.kind()),
        Var::Layer => borrowed(s.layer()),
        Var::Label => s.label().map_or(Value::Null, borrowed),
        Var::Index => Value::Num(s.index()),
        Var::Id => Value::Num(s.id()),
        Var::Scale => opt(s.scale()),
        Var::CentroidY => opt(s.geometry(Geometry::CentroidY)),
        Var::CentroidX => opt(s.geometry(Geometry::CentroidX)),
        Var::MinY => opt(s.geometry(Geometry::MinY)),
        Var::MaxY => opt(s.geometry(Geometry::MaxY)),
        Var::MinX => opt(s.geometry(Geometry::MinX)),
        Var::MaxX => opt(s.geometry(Geometry::MaxX)),
        Var::Width => opt(s.geometry(Geometry::Width)),
        Var::Height => opt(s.geometry(Geometry::Height)),
    }
}

/// What a result of the rules is, found while the arguments are still read.
enum Got {
    Value(Value<'static>),
    /// Text the rules made (in `made`).
    Made,
    Thrown,
    /// Text that is bytes `i..j` of argument `k`.
    Part {
        k: usize,
        i: usize,
        j: usize,
    },
}

/// A result of the rules: text that is part of an argument is found by its
/// address, so it can be taken from that argument (borrowed where it was).
fn got(r: R, args: &[Value]) -> Got {
    let t = match r {
        R::Made => return Got::Made,
        R::Thrown => return Got::Thrown,
        R::V(V::Null) => return Got::Value(Value::Null),
        R::V(V::Num(x)) => return Got::Value(Value::Num(x)),
        R::V(V::Bool(b)) => return Got::Value(Value::Bool(b)),
        R::V(V::Text(t)) => t,
    };
    let (from, to) = (t.as_ptr() as usize, t.as_ptr() as usize + t.len());
    for (k, a) in args.iter().enumerate() {
        let Value::Text(s) = a else { continue };
        let start = s.as_ptr() as usize;
        if start <= from && to <= start + s.len() {
            return Got::Part {
                k,
                i: from - start,
                j: to - start,
            };
        }
    }
    // Not an argument's: one of the language's own words.
    Got::Value(Value::Text(match t {
        "doğru" => Cow::Borrowed("doğru"),
        "yanlış" => Cow::Borrowed("yanlış"),
        "" => Cow::Borrowed(""),
        t => Cow::Owned(t.to_string()),
    }))
}

fn value<'a>(g: Got, made: String, args: &mut [Value<'a>]) -> Result<Value<'a>, Thrown> {
    Ok(match g {
        Got::Value(v) => v,
        Got::Made => Value::Text(Cow::Owned(made)),
        Got::Thrown => return Err(Thrown),
        Got::Part { k, i, j } => match args.get_mut(k).map(std::mem::take) {
            Some(Value::Text(Cow::Borrowed(b))) => borrowed(b.get(i..j).unwrap_or_default()),
            Some(Value::Text(Cow::Owned(o))) if i == 0 && j == o.len() => {
                Value::Text(Cow::Owned(o))
            }
            Some(Value::Text(Cow::Owned(o))) => {
                Value::Text(Cow::Owned(o.get(i..j).unwrap_or_default().to_string()))
            }
            _ => Value::Null,
        },
    })
}

/// A function's value on its arguments, by the rules.
fn apply<'a>(
    f: crate::library::Func,
    args: &mut [Value<'a>],
    sc: &mut Option<Scratch>,
) -> Result<Value<'a>, Thrown> {
    let mut made = String::new();
    let g = {
        const FEW: usize = 8;
        let mut few = [V::Null; FEW];
        let many: Vec<V>;
        let views: &[V] = if args.len() <= FEW {
            for (v, a) in few.iter_mut().zip(args.iter()) {
                *v = a.view();
            }
            &few[..args.len()]
        } else {
            many = args.iter().map(Value::view).collect();
            &many
        };
        got(functions::call(f, views, &mut made, buffers(sc)), args)
    };
    value(g, made, args)
}

/// An operator on two finite numbers, directly (the rules give the same);
/// None where the rules must decide (a side that is not finite, `||`).
#[inline]
fn numbers(op: BinOp, x: f64, y: f64) -> Option<Value<'static>> {
    if !(x.is_finite() && y.is_finite()) {
        return None;
    }
    let close = || {
        (x - y).abs()
            <= 1e-9
                * kentos_geometry_core::jsmath::js_max(
                    kentos_geometry_core::jsmath::js_max(1.0, x.abs()),
                    y.abs(),
                )
    };
    Some(match op {
        BinOp::Add => Value::Num(x + y),
        BinOp::Sub => Value::Num(x - y),
        BinOp::Mul => Value::Num(x * y),
        BinOp::Div | BinOp::Rem if y == 0.0 => Value::Null,
        BinOp::Div => Value::Num(x / y),
        BinOp::Rem => Value::Num(x % y),
        BinOp::Eq => Value::Bool(close()),
        BinOp::Ne => Value::Bool(!close()),
        BinOp::Lt => Value::Bool(x - y < 0.0),
        BinOp::Le => Value::Bool(x - y <= 0.0),
        BinOp::Gt => Value::Bool(x - y > 0.0),
        BinOp::Ge => Value::Bool(x - y >= 0.0),
        BinOp::And => Value::Bool(x != 0.0 && y != 0.0),
        BinOp::Or => Value::Bool(x != 0.0 || y != 0.0),
        BinOp::Join => return None,
    })
}

/// The rules' buffers, made the first time an operation needs them.
fn buffers(sc: &mut Option<Scratch>) -> &mut Scratch {
    sc.get_or_insert_with(Scratch::default)
}

/// The value of `n` for the object `s`; `sc` holds the rules' buffers once
/// an operation needed them.
pub fn walk<'a>(
    n: &'a Node,
    s: &'a dyn Scope,
    sc: &mut Option<Scratch>,
) -> Result<Value<'a>, Thrown> {
    Ok(match n {
        Node::Lit(Value::Text(t)) => borrowed(t),
        Node::Lit(v) => v.clone(),
        Node::Field(i) => s.field(*i).map_or(Value::Null, borrowed),
        Node::Var(v) => variable(*v, s),
        Node::Not(a) => Value::Bool(!scalar::truthy(walk(a, s, sc)?.view())),
        Node::Neg(a) => match scalar::neg(walk(a, s, sc)?.view()) {
            V::Num(x) => Value::Num(x),
            _ => Value::Null,
        },
        Node::Bin(op, a, b) => {
            let a = walk(a, s, sc)?;
            let b = walk(b, s, sc)?;
            if let (Value::Num(x), Value::Num(y)) = (&a, &b)
                && let Some(v) = numbers(*op, *x, *y)
            {
                return Ok(v);
            }
            if matches!(op, BinOp::And | BinOp::Or) {
                let (p, q) = (scalar::truthy(a.view()), scalar::truthy(b.view()));
                return Ok(Value::Bool(if *op == BinOp::And { p && q } else { p || q }));
            }
            if *op == BinOp::Join
                && let Value::Text(Cow::Owned(mut left)) = a
            {
                // Text the walk made is extended in place, as before the column engine.
                let right = b.view();
                let bound = left.len() + if let V::Text(t) = right { t.len() } else { 32 };
                if bound <= scalar::MAX_JOIN_BYTES {
                    scalar::push_text(&mut left, right);
                    return Ok(Value::Text(Cow::Owned(left)));
                }
                let mut made = String::new();
                return match scalar::binary(*op, V::Text(&left), right, &mut made, buffers(sc)) {
                    R::Made => Ok(Value::Text(Cow::Owned(made))),
                    _ => Err(Thrown),
                };
            }
            let mut made = String::new();
            // An operator never gives back an argument's text.
            match scalar::binary(*op, a.view(), b.view(), &mut made, buffers(sc)) {
                R::V(V::Num(x)) => Value::Num(x),
                R::V(V::Bool(b)) => Value::Bool(b),
                R::Made => Value::Text(Cow::Owned(made)),
                R::Thrown => return Err(Thrown),
                R::V(V::Null | V::Text(_)) => Value::Null,
            }
        }
        Node::Call(f, args) if args.len() <= 4 => {
            // Most calls take a few arguments: they stay on the stack.
            let mut values: [Value<'a>; 4] = Default::default();
            let mut xs = [0.0; 4];
            let mut numbers = true;
            for ((slot, x), a) in values.iter_mut().zip(xs.iter_mut()).zip(args) {
                *slot = walk(a, s, sc)?;
                match slot {
                    Value::Num(v) if v.is_finite() => *x = *v,
                    _ => numbers = false,
                }
            }
            if numbers && let Some(v) = functions::number_function(*f, &xs[..args.len()]) {
                return Ok(match v {
                    V::Num(x) => Value::Num(x),
                    _ => Value::Null,
                });
            }
            apply(*f, &mut values[..args.len()], sc)?
        }
        Node::Call(f, args) => {
            let mut values = args
                .iter()
                .map(|a| walk(a, s, sc))
                .collect::<Result<Vec<_>, _>>()?;
            apply(*f, &mut values, sc)?
        }
    })
}
