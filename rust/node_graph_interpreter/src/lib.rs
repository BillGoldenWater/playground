use std::{fmt::Debug, hash::Hash, sync::Arc};

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

pub trait Exec: Debug {
    /// # Returns
    /// next branch index, ignored in non exec node
    fn exec(
        &self,
        ctx: &mut Context,
        params: &[Value],
        output: &mut Vec<Value>,
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
        next: Box<[Box<[FlowIndexes]>]>,

        exec: Arc<dyn Exec>,
    },
    Operation {
        parameters: Box<[ParameterIndexes]>,

        exec: Arc<dyn Exec>,
    },
}

#[derive(Debug, Default)]
pub struct Context {
    pub nodes: Arc<[Node]>,
    pub values: Box<[Option<Vec<Value>>]>,
    pub local_variables: Vec<Value>,
    pub outputs: Vec<Vec<Value>>,
    // TODO: , and clear
    // loop_flags: HashMap<usize, bool>,
}

impl Context {
    pub fn run_start(
        &mut self,
        nodes: Arc<[Node]>,
        idx: usize,
        values: Vec<Value>,
    ) {
        self.nodes = nodes.clone();
        // TODO: in-place clear
        self.values = vec![None; nodes.len()].into_boxed_slice();
        self.local_variables = Vec::with_capacity(8);
        self.outputs = Vec::with_capacity(8);

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
                    let params = parameters.clone();
                    let exec = exec.clone();
                    let mut output = self.value_cache_get();

                    let mut params_out = self.value_cache_get();
                    self.query_params(&params, &mut params_out);
                    let branch_idx =
                        exec.exec(self, &params_out, &mut output);

                    // TODO: in-place update if possible
                    if let Some(values) = &mut self.values[idx] {
                        values.clear();
                        values.extend_from_slice(&output);
                        self.value_cache_ret(output);
                    } else {
                        self.values[idx] = Some(output)
                    }
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
        self.outputs = Vec::with_capacity(8);

        let Node::End { parameters } = &nodes[idx] else {
            panic!("expect end node");
        };

        let mut output = self.value_cache_get();
        self.query_params(parameters, &mut output);
        output.into_boxed_slice()
    }

    pub fn query_params(
        &mut self,
        params: &[ParameterIndexes],
        params_out: &mut Vec<Value>,
    ) {
        let nodes = self.nodes.clone();
        params_out.extend(
            params
                .iter()
                .map(|param_idx| self.query_param(&nodes, param_idx)),
        );
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
                let mut output = self.value_cache_get();

                let mut params_out = self.value_cache_get();
                self.query_params(parameters, &mut params_out);

                exec.exec(self, &params_out, &mut output);
                let ret = output[param_idx.value].clone();

                self.value_cache_ret(output);

                ret
            }
            Node::End { .. } => {
                panic!("expect node that may output value")
            }
        }
    }

    pub fn value_cache_get(&mut self) -> Vec<Value> {
        self.outputs.pop().unwrap_or_else(|| Vec::with_capacity(8))
    }

    pub fn value_cache_ret(&mut self, mut value: Vec<Value>) {
        value.clear();
        self.outputs.push(value);
    }

    pub fn get_local_variable(&mut self, key: usize) -> &mut Value {
        if key >= self.local_variables.len() {
            self.local_variables.resize(key + 1, Value::Uninit);
        }

        &mut self.local_variables[key]
    }

    // pub fn list_assemble(&mut self, values: Vec<Value>) -> Value {
    //     self.lists.push(values);
    //     Value::List(self.lists.len() - 1)
    // }
    //
    // pub fn list_get(&mut self, list: usize, idx: usize) -> Value {
    //     self.lists[list][idx].clone()
    // }
    //
    // pub fn list_set(&mut self, list: usize, idx: usize, value: Value) {
    //     self.lists[list][idx] = value;
    // }
    //
    // pub fn list_len(&mut self, list: usize) -> usize {
    //     self.lists[list].len()
    // }

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
