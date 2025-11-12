use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use crate::value::Value;

pub mod nodes;
pub mod value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParameterIndexes {
    pub node: usize,
    pub value: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlowIndexes {
    pub node: usize,
}

pub trait BehaviorExec: Debug {
    fn exec(
        &self,
        ctx: &mut Context,
        params: &[ParameterIndexes],
    ) -> Box<[Value]> {
        let params = ctx.query_params(params);
        self.exec_with_param(ctx, params)
    }

    fn exec_with_param(
        &self,
        ctx: &mut Context,
        params: Box<[Value]>,
    ) -> Box<[Value]>;
}

pub trait BehaviorBranch: Debug {
    fn branch(
        &self,
        ctx: &mut Context,
        params: &[ParameterIndexes],
    ) -> usize {
        let params = ctx.query_params(params);
        self.branch_with_param(ctx, params)
    }

    fn branch_with_param(
        &self,
        ctx: &mut Context,
        params: Box<[Value]>,
    ) -> usize;
}

#[derive(Debug, Clone)]
pub enum Node {
    Constant {
        values: Box<[Value]>,
    },
    Start {
        next: Box<[FlowIndexes]>,
    },
    End {
        parameters: Box<[ParameterIndexes]>,
    },
    Exec {
        parameters: Box<[ParameterIndexes]>,
        next: Box<[FlowIndexes]>,

        exec: Arc<dyn BehaviorExec>,
    },
    FlowControl {
        parameters: Box<[ParameterIndexes]>,
        next: Box<[Box<[FlowIndexes]>]>,

        branch: Arc<dyn BehaviorBranch>,
    },
    Operation {
        parameters: Box<[ParameterIndexes]>,

        exec: Arc<dyn BehaviorExec>,
    },
}

#[derive(Debug, Default)]
pub struct Context {
    pub nodes: Arc<[Node]>,
    pub values: Box<[Option<Box<[Value]>>]>,
    pub local_variables: HashMap<usize, Value>,
    pub lists: Vec<Vec<Value>>,
    // TODO: , and clear
    // loop_flags: HashMap<usize, bool>,
}

impl Context {
    pub fn run_start(
        &mut self,
        nodes: Arc<[Node]>,
        idx: usize,
        values: Box<[Value]>,
    ) {
        self.nodes = nodes.clone();
        // TODO: in-place clear
        self.values = vec![None; nodes.len()].into_boxed_slice();
        self.local_variables = Default::default();
        self.lists = Vec::with_capacity(8);

        let Node::Start { next } = &nodes[idx] else {
            panic!("expect start node");
        };

        self.values[idx] = Some(values);

        for idx in next.iter().rev() {
            self.run_inner(idx.node);
        }
    }

    pub fn run_inner(&mut self, idx: usize) {
        let nodes = self.nodes.clone();
        let mut exec_queue = Vec::<usize>::with_capacity(8);
        exec_queue.push(idx);

        while let Some(idx) = exec_queue.pop() {
            match &nodes[idx] {
                Node::Exec {
                    parameters,
                    exec,
                    next,
                } => {
                    let param = parameters.clone();
                    let exec = exec.clone();

                    let res = exec.exec(self, &param);

                    self.values[idx] = Some(res);
                    exec_queue
                        .extend(next.iter().rev().map(|it| it.node));
                }
                Node::FlowControl {
                    parameters,
                    branch,
                    next,
                } => {
                    let param = parameters.clone();
                    let branch = branch.clone();

                    let branch_idx = branch.branch(self, &param);

                    exec_queue.extend(
                        next[branch_idx].iter().rev().map(|it| it.node),
                    );
                }
                Node::Start { .. }
                | Node::End { .. }
                | Node::Operation { .. }
                | Node::Constant { .. } => {
                    panic!("expect executable nodes");
                }
            }
        }
    }

    pub fn run_end(
        &mut self,
        nodes: Arc<[Node]>,
        idx: usize,
    ) -> Box<[Value]> {
        self.nodes = nodes.clone();
        // TODO: in-place clear
        self.values = vec![None; nodes.len()].into_boxed_slice();
        self.local_variables = Default::default();

        let Node::End { parameters } = &nodes[idx] else {
            panic!("expect end node");
        };

        self.query_params(parameters)
    }

    pub fn query_params(
        &mut self,
        params: &[ParameterIndexes],
    ) -> Box<[Value]> {
        let nodes = self.nodes.clone();
        params
            .iter()
            .map(|param_idx| self.query_param(&nodes, param_idx))
            .collect()
    }

    pub fn query_param(
        &mut self,
        nodes: &[Node],
        param_idx: &ParameterIndexes,
    ) -> Value {
        match &nodes[param_idx.node] {
            Node::Constant { values } => values[param_idx.value].clone(),
            Node::Start { .. } | Node::Exec { .. } => self.values
                [param_idx.node]
                .as_ref()
                .map(|it| it[param_idx.value].clone())
                .unwrap_or_default(),
            Node::Operation { parameters, exec } => {
                exec.exec(self, parameters)[param_idx.value].clone()
            }
            Node::End { .. } | Node::FlowControl { .. } => {
                panic!("expect node that may output value")
            }
        }
    }

    pub fn get_local_variable(&mut self, key: usize) -> &mut Value {
        self.local_variables.entry(key).or_insert(Value::Uninit)
    }

    pub fn list_assemble(&mut self, values: Vec<Value>) -> Value {
        self.lists.push(values);
        Value::List(self.lists.len() - 1)
    }

    pub fn list_get(&mut self, list: usize, idx: usize) -> Value {
        self.lists[list][idx].clone()
    }

    pub fn list_set(&mut self, list: usize, idx: usize, value: Value) {
        self.lists[list][idx] = value;
    }

    pub fn list_len(&mut self, list: usize) -> usize {
        self.lists[list].len()
    }

    // pub fn loop_begin(&mut self, loop_id: usize) {
    //     self.loop_flags
    //         .entry(loop_id)
    //         .and_modify(|it| *it = false)
    //         .or_insert(false);
    // }
    //
    // pub fn loop_is_breaking(&mut self, loop_id: usize) -> bool {
    //     *self.loop_flags.get(&loop_id).expect("expect valid loop_id")
    // }
    //
    // pub fn loop_break(&mut self, loop_id: usize) {
    //     self.loop_flags.entry(loop_id).and_modify(|it| *it = true);
    // }
}
