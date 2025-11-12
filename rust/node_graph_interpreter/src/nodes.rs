use crate::{
    BehaviorBranch, BehaviorExec, Context, ParameterIndexes, Value,
};

#[derive(Debug)]
pub struct Noop;

impl BehaviorExec for Noop {
    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        _params: Box<[Value]>,
    ) -> Box<[Value]> {
        Box::new([])
    }
}

#[derive(Debug)]
pub struct LocalVariable;

impl BehaviorExec for LocalVariable {
    fn exec(
        &self,
        ctx: &mut Context,
        params: &[ParameterIndexes],
    ) -> Box<[crate::value::Value]> {
        debug_assert!((1..=2).contains(&params.len()));

        let nodes = ctx.nodes.clone();

        let var_key = ctx.query_param(&nodes, &params[0]);
        let var = ctx.get_local_variable(var_key.as_local_variable());

        if var.is_uninit() {
            let init_value = ctx.query_param(&nodes, &params[1]);
            let var = ctx.get_local_variable(var_key.as_local_variable());
            *var = init_value;
            Box::new([var.clone()])
        } else {
            Box::new([var.clone()])
        }
    }

    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        _params: Box<[Value]>,
    ) -> Box<[Value]> {
        unreachable!()
    }
}

#[derive(Debug)]
pub struct LocalVariableSet;

impl BehaviorExec for LocalVariableSet {
    fn exec(
        &self,
        ctx: &mut Context,
        params: &[ParameterIndexes],
    ) -> Box<[crate::value::Value]> {
        debug_assert_eq!(params.len(), 2);

        let params = ctx.query_params(params);
        let var = ctx.get_local_variable(params[0].as_local_variable());

        *var = params[1].clone();

        Box::new([])
    }

    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        _params: Box<[Value]>,
    ) -> Box<[Value]> {
        unreachable!()
    }
}

#[derive(Debug)]
pub struct ListAssemble;

impl BehaviorExec for ListAssemble {
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        Box::new([ctx.list_assemble(params.into_vec())])
    }
}

#[derive(Debug)]
pub struct ListGet;

impl BehaviorExec for ListGet {
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 2);

        Box::new([ctx
            .list_get(params[0].as_list(), params[1].as_int() as usize)])
    }
}

#[derive(Debug)]
pub struct ListSet;

impl BehaviorExec for ListSet {
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 3);

        ctx.list_set(
            params[0].as_list(),
            params[1].as_int() as usize,
            params[2].clone(),
        );

        Box::new([])
    }
}

#[derive(Debug)]
pub struct ListLength;

impl BehaviorExec for ListLength {
    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 1);

        Box::new([Value::Int(ctx.list_len(params[0].as_list()) as i64)])
    }
}

#[derive(Debug)]
pub struct Addition;

impl BehaviorExec for Addition {
    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 2);

        Box::new([Value::Int(params[0].as_int() + params[1].as_int())])
    }
}

#[derive(Debug)]
pub struct Subtraction;

impl BehaviorExec for Subtraction {
    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 2);

        Box::new([Value::Int(params[0].as_int() - params[1].as_int())])
    }
}

#[derive(Debug)]
pub struct IsGreaterThan;

impl BehaviorExec for IsGreaterThan {
    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 2);

        Box::new([Value::Bool(params[0].as_int() > params[1].as_int())])
    }
}

#[derive(Debug)]
pub struct IsLessThan;

impl BehaviorExec for IsLessThan {
    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 2);

        Box::new([Value::Bool(params[0].as_int() < params[1].as_int())])
    }
}

#[derive(Debug)]
pub struct Print;

impl BehaviorExec for Print {
    fn exec_with_param(
        &self,
        _ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]> {
        debug_assert_eq!(params.len(), 1);

        println!("{}", params[0].as_str());

        Box::new([])
    }
}

#[derive(Debug)]
pub struct DoubleBranch;

impl BehaviorBranch for DoubleBranch {
    fn branch_with_param(
        &self,
        _ctx: &mut Context,
        params: Box<[Value]>,
    ) -> usize {
        debug_assert_eq!(params.len(), 1);

        if params[0].as_bool() { 0 } else { 1 }
    }
}
