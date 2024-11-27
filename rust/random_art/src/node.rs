use core::f64;
use std::ops::{Add, Div, Mul, Sub};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    X,
    Y,
    Lit(f64),
    Rgb(Box<Node>, Box<Node>, Box<Node>),

    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),
    Mod(Box<Node>, Box<Node>),
    Pow(Box<Node>, Box<Node>),
    Sin(Box<Node>),
    Cos(Box<Node>),
    Exp(Box<Node>),
    Sqrt(Box<Node>),
    Mix(Box<Node>, Box<Node>, Box<Node>, Box<Node>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Value {
    Single(f64),
    Rgb(f64, f64, f64),
}

impl Value {
    pub fn binary_op(
        self,
        rhs: Self,
        op: impl Fn(f64, f64) -> f64,
    ) -> Self {
        let do_rgb = |a: (f64, f64, f64), b: (f64, f64, f64)| {
            Value::Rgb(op(a.0, b.0), op(a.1, b.1), op(a.2, b.2))
        };

        match (self, rhs) {
            (Value::Single(a), Value::Single(b)) => {
                Self::Single(op(a, b))
            }
            (Value::Single(a), Value::Rgb(b1, b2, b3)) => {
                do_rgb((a, a, a), (b1, b2, b3))
            }
            (Value::Rgb(a1, a2, a3), Value::Single(b)) => {
                do_rgb((a1, a2, a3), (b, b, b))
            }
            (Value::Rgb(a1, a2, a3), Value::Rgb(b1, b2, b3)) => {
                do_rgb((a1, a2, a3), (b1, b2, b3))
            }
        }
    }

    pub fn unary_op(self, op: impl Fn(f64) -> f64) -> Self {
        self.binary_op(Self::Single(0.0), |a, _| op(a))
    }

    pub fn fmod(self, rhs: Self) -> Self {
        self.binary_op(rhs, |a, b| {
            if b != 0.0 {
                a % b
            } else {
                a % f64::EPSILON
            }
        })
    }

    pub fn sin(self) -> Self {
        self.unary_op(|a| a.sin())
    }

    pub fn cos(self) -> Self {
        self.unary_op(|a| a.cos())
    }

    pub fn abs(self) -> Self {
        self.unary_op(|a| a.abs())
    }

    pub fn exp(self) -> Self {
        self.unary_op(|a| a.exp())
    }

    pub fn sqrt(self) -> Self {
        self.unary_op(|a| a.sqrt())
    }
}

impl Value {
    pub fn to_rgb(self) -> [f64; 3] {
        match self {
            Value::Single(luma) => [luma, luma, luma],
            Value::Rgb(r, g, b) => [r, g, b],
        }
    }

    pub fn to_rgb8(self) -> [u8; 3] {
        let [r, g, b] = self.to_rgb();

        [to_luma(r), to_luma(g), to_luma(b)]
    }

    pub fn to_argb8(self) -> [u8; 4] {
        let [r, g, b] = self.to_rgb();

        [u8::MAX, to_luma(r), to_luma(g), to_luma(b)]
    }

    pub fn to_single(self) -> f64 {
        match self {
            Value::Single(v) => v,
            Value::Rgb(r, g, b) => (r + g + b) / 3.0,
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Single(value)
    }
}

impl From<[f64; 3]> for Value {
    fn from(value: [f64; 3]) -> Self {
        Self::Rgb(value[0], value[1], value[2])
    }
}

impl Add for Value {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.binary_op(rhs, f64::add)
    }
}

impl<T: Into<Self>> Sub<T> for Value {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        self.binary_op(rhs.into(), f64::sub)
    }
}

impl<T: Into<Self>> Mul<T> for Value {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        self.binary_op(rhs.into(), f64::mul)
    }
}

impl<T: Into<Self>> Div<T> for Value {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        self.binary_op(rhs.into(), |a, b| {
            if b != 0.0 {
                a / b
            } else {
                a / f64::EPSILON
            }
        })
    }
}

impl Node {
    pub fn eval(&self, x: f64, y: f64) -> Value {
        match self {
            Node::X => Value::Single(x),
            Node::Y => Value::Single(y),
            Node::Lit(v) => (*v).into(),
            Node::Rgb(a, b, c) => {
                let r = a.eval(x, y);
                let g = b.eval(x, y);
                let b = c.eval(x, y);

                Value::Rgb(r.to_single(), g.to_single(), b.to_single())
            }

            Node::Add(a, b) => (a.eval(x, y) + b.eval(x, y)) / 2.0,
            Node::Sub(a, b) => (a.eval(x, y) - b.eval(x, y)) / 2.0,
            Node::Mul(a, b) => a.eval(x, y) * b.eval(x, y),
            Node::Div(a, b) => {
                let b = b.eval(x, y);

                (a.eval(x, y) / b).unary_op(clamp)
            }
            Node::Mod(a, b) => a.eval(x, y).fmod(b.eval(x, y)),
            Node::Pow(a, b) => {
                a.eval(x, y).binary_op(b.eval(x, y), |a, b| a.powf(b))
            }
            Node::Sin(a) => a.eval(x, y).sin(),
            Node::Cos(a) => a.eval(x, y).cos(),
            Node::Exp(a) => {
                const K: f64 = 1.0;
                let a = a.eval(x, y);
                let b = (-K).exp();

                (a.exp() - b) / (K.exp() - b)
                //a.eval(x, y).exp().unary_op(clamp)
            }
            Node::Sqrt(a) => a.eval(x, y).abs().sqrt() * 2.0 - 1.0,
            Node::Mix(a, b, c, d) => {
                let a = a.eval(x, y);
                let b = b.eval(x, y);
                let c = c.eval(x, y);
                let d = d.eval(x, y);

                let g = a * b;

                (Value::from(1.0) - g) * c + g * d
            }
        }
    }
}

pub fn clamp(x: f64) -> f64 {
    x.clamp(-1.0, 1.0)
}

pub fn to_luma(x: f64) -> u8 {
    ((x + 1.0) / 2.0 * 255.0).round() as u8
}
