use std::{cell::RefCell, sync::Arc};

use crate::{Context, Exec, Value};

#[derive(Debug)]
pub struct Noop;

impl Exec for Noop {
    fn exec(
        &self,
        _ctx: &mut Context,
        _params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        0
    }
}

#[derive(Debug)]
pub struct LocalVariable;

impl Exec for LocalVariable {
    fn exec(
        &self,
        ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert!((1..=2).contains(&params.len()));

        let var_key = &params[0];
        let var = ctx.get_local_variable(var_key.as_local_variable());

        if var.is_uninit() {
            *var = params[1].clone();
            output.push(var.clone());
        } else {
            output.push(var.clone());
        }

        0
    }
}

#[derive(Debug)]
pub struct LocalVariableSet;

impl Exec for LocalVariableSet {
    fn exec(
        &self,
        ctx: &mut Context,
        params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 2);

        let var = ctx.get_local_variable(params[0].as_local_variable());

        *var = params[1].clone();

        0
    }
}

#[derive(Debug)]
pub struct ListAssemble;

impl Exec for ListAssemble {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        output.push(Value::List(Arc::new(RefCell::new(params.to_vec()))));

        0
    }
}

#[derive(Debug)]
pub struct ListGet;

impl Exec for ListGet {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 2);

        output.push(
            params[0].as_list().borrow()[params[1].as_int() as usize]
                .clone(),
        );

        0
    }
}

#[derive(Debug)]
pub struct ListSet;

impl Exec for ListSet {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 3);

        params[0].as_list().borrow_mut()[params[1].as_int() as usize] =
            params[2].clone();

        0
    }
}

#[derive(Debug)]
pub struct ListLength;

impl Exec for ListLength {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 1);

        output
            .push(Value::Int(params[0].as_list().borrow().len() as i64));

        0
    }
}

#[derive(Debug)]
pub struct Addition;

impl Exec for Addition {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 2);

        output.push(Value::Int(params[0].as_int() + params[1].as_int()));

        0
    }
}

#[derive(Debug)]
pub struct Subtraction;

impl Exec for Subtraction {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 2);

        output.push(Value::Int(params[0].as_int() - params[1].as_int()));

        0
    }
}

#[derive(Debug)]
pub struct IsGreaterThan;

impl Exec for IsGreaterThan {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 2);

        output.push(Value::Bool(params[0].as_int() > params[1].as_int()));

        0
    }
}

#[derive(Debug)]
pub struct IsLessThan;

impl Exec for IsLessThan {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 2);

        output.push(Value::Bool(params[0].as_int() < params[1].as_int()));

        0
    }
}

#[derive(Debug)]
pub struct Print;

impl Exec for Print {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 1);

        println!("{}", params[0].as_str());

        0
    }
}

#[derive(Debug)]
pub struct DoubleBranch;

impl Exec for DoubleBranch {
    fn exec(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 1);

        if params[0].as_bool() { 0 } else { 1 }
    }
}
