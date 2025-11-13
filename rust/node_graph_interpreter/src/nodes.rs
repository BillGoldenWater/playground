use std::{cell::RefCell, rc::Rc};

use crate::{Context, Exec, ParameterIndexes, Value};

#[derive(Debug)]
pub struct Noop;

impl Exec for Noop {
    fn exec(
        &self,
        _ctx: &mut Context,
        _stack: &mut Vec<Value>,
        _param_base: usize,
    ) -> usize {
        0
    }
}

#[derive(Debug)]
pub struct LocalVariable;

impl Exec for LocalVariable {
    fn exec(
        &self,
        _ctx: &mut Context,
        _stack: &mut Vec<Value>,
        _param_base: usize,
    ) -> usize {
        unreachable!()
    }

    fn manual_param(&self) -> bool {
        true
    }

    fn exec_manual_param(
        &self,
        ctx: &mut Context,
        params: &[ParameterIndexes],
        stack: &mut Vec<Value>,
    ) -> usize {
        debug_assert!((1..=2).contains(&params.len()));

        let param_base = stack.len();
        ctx.query_params(&params[..1], stack);

        let var_key = &stack[param_base];
        let var = ctx.get_local_variable(var_key.as_local_variable());

        if var.is_uninit() {
            ctx.query_params(&params[1..2], stack);
            let var_key = &stack[param_base];
            let var = ctx.get_local_variable(var_key.as_local_variable());
            *var = stack[param_base + 1].clone();
            stack[param_base] = var.clone();
            stack.pop();
        } else {
            stack[param_base] = var.clone()
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
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 2);

        let value = stack.pop().expect("expect 2 parameters");
        let var = stack.pop().expect("expect 2 parameters");

        let var = ctx.get_local_variable(var.as_local_variable());

        *var = value;

        0
    }
}

#[derive(Debug)]
pub struct ListAssemble;

impl Exec for ListAssemble {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        let list = stack[param_base..].to_vec();
        stack.truncate(param_base);
        stack.push(Value::List(Rc::new(RefCell::new(list))));

        0
    }
}

#[derive(Debug)]
pub struct ListGet;

impl Exec for ListGet {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 2);

        let idx = stack.pop().expect("expect 2 parameters");
        let list = stack.pop().expect("expect 2 parameters");

        let value =
            list.as_list().borrow()[idx.as_int() as usize].clone();

        stack.push(value);

        0
    }
}

#[derive(Debug)]
pub struct ListSet;

impl Exec for ListSet {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 3);

        let value = stack.pop().expect("expect 3 parameters");
        let idx = stack.pop().expect("expect 3 parameters");
        let list = stack.pop().expect("expect 3 parameters");

        list.as_list().borrow_mut()[idx.as_int() as usize] = value;

        0
    }
}

#[derive(Debug)]
pub struct ListLength;

impl Exec for ListLength {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 1);

        let list = stack.pop().expect("expect 1 parameter");
        stack.push(Value::Int(list.as_list().borrow().len() as i64));

        0
    }
}

#[derive(Debug)]
pub struct Addition;

impl Exec for Addition {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 2);

        let b = stack.pop().expect("expect 2 parameters");
        let a = stack.pop().expect("expect 2 parameters");
        stack.push(Value::Int(a.as_int() + b.as_int()));

        0
    }
}

#[derive(Debug)]
pub struct Subtraction;

impl Exec for Subtraction {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 2);

        let b = stack.pop().expect("expect 2 parameters");
        let a = stack.pop().expect("expect 2 parameters");
        stack.push(Value::Int(a.as_int() - b.as_int()));

        0
    }
}

#[derive(Debug)]
pub struct IsGreaterThan;

impl Exec for IsGreaterThan {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 2);

        let b = stack.pop().expect("expect 2 parameters");
        let a = stack.pop().expect("expect 2 parameters");
        stack.push(Value::Bool(a.as_int() > b.as_int()));

        0
    }
}

#[derive(Debug)]
pub struct IsLessThan;

impl Exec for IsLessThan {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 2);

        let b = stack.pop().expect("expect 2 parameters");
        let a = stack.pop().expect("expect 2 parameters");
        stack.push(Value::Bool(a.as_int() < b.as_int()));

        0
    }
}

#[derive(Debug)]
pub struct Print;

impl Exec for Print {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 1);

        println!("{}", stack.pop().expect("expect 1 parameter").as_str());

        0
    }
}

#[derive(Debug)]
pub struct DoubleBranch;

impl Exec for DoubleBranch {
    fn exec(
        &self,
        _ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize {
        debug_assert_eq!(stack.len() - param_base, 1);

        let a = stack.pop().expect("expect 1 parameter");

        if a.as_bool() { 0 } else { 1 }
    }
}
