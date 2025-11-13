use crate::{Context, Exec, ParameterIndexes, Value};

#[derive(Debug)]
pub struct Noop;

impl Exec for Noop {
    fn exec_with_param(
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
        params: &[ParameterIndexes],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert!((1..=2).contains(&params.len()));

        let nodes = ctx.nodes.clone();

        let var_key = ctx.query_param(&nodes, &params[0]);
        let var = ctx.get_local_variable(var_key.as_local_variable());

        if var.is_uninit() {
            let init_value = ctx.query_param(&nodes, &params[1]);
            let var = ctx.get_local_variable(var_key.as_local_variable());
            *var = init_value;
            output.push(var.clone());
        } else {
            output.push(var.clone());
        }

        0
    }

    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        _params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        unreachable!()
    }
}

#[derive(Debug)]
pub struct LocalVariableSet;

impl Exec for LocalVariableSet {
    fn exec_with_param(
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
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        output.push(ctx.list_assemble(params.to_vec()));

        0
    }
}

#[derive(Debug)]
pub struct ListGet;

impl Exec for ListGet {
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 2);

        output.push(
            ctx.list_get(
                params[0].as_list(),
                params[1].as_int() as usize,
            ),
        );

        0
    }
}

#[derive(Debug)]
pub struct ListSet;

impl Exec for ListSet {
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 3);

        ctx.list_set(
            params[0].as_list(),
            params[1].as_int() as usize,
            params[2].clone(),
        );

        0
    }
}

#[derive(Debug)]
pub struct ListLength;

impl Exec for ListLength {
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 1);

        output.push(Value::Int(ctx.list_len(params[0].as_list()) as i64));

        0
    }
}

#[derive(Debug)]
pub struct Addition;

impl Exec for Addition {
    fn exec_with_param(
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
    fn exec_with_param(
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
    fn exec_with_param(
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
    fn exec_with_param(
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
    fn exec_with_param(
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
    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        params: &[Value],
        _output: &mut Vec<Value>,
    ) -> usize {
        debug_assert_eq!(params.len(), 1);

        if params[0].as_bool() { 0 } else { 1 }
    }
}
