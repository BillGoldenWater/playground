use core::panic;
use std::{
    fmt::Debug,
    hash::Hash,
    sync::{
        Arc,
        atomic::{self, AtomicU32},
    },
};

use crate::value::Value;

pub static COUNT: AtomicU32 = AtomicU32::new(0);

pub mod nodes;
pub mod value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterIndexes {
    pub node: usize,
    pub value: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlowIndexes {
    pub node: usize,
}

pub trait Exec: Debug {
    /// `param_base` points to the first parameter in stack,
    /// equal to stack.len() if empty parameter
    ///
    /// implementation need to consume all the parameter,
    /// then push output
    /// # Returns
    /// next branch index, ignored in non exec node
    fn exec(
        &self,
        ctx: &mut Context,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize;

    fn manual_param(&self) -> bool {
        false
    }

    /// fetch parameters manually
    /// # Returns
    /// next branch index, ignored in non exec node
    fn exec_manual_param(
        &self,
        ctx: &mut Context,
        params: &[ParameterIndexes],
        stack: &mut Vec<Value>,
    ) -> usize {
        let (..) = (ctx, params, stack);
        unreachable!();
    }
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

#[derive(Debug, Clone, Copy)]
pub struct PendingParam {
    idx: ParameterIndexes,
    visited: bool,
}

impl From<ParameterIndexes> for PendingParam {
    fn from(value: ParameterIndexes) -> Self {
        Self {
            idx: value,
            visited: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Context {
    pub nodes: Arc<[Node]>,
    pub values: Box<[Option<Vec<Value>>]>,
    pub local_variables: Vec<Value>,
    pub value_cache: Vec<Vec<Value>>,
    pub pending_param_cache: Vec<Vec<PendingParam>>,
    // TODO: , and clear
    // loop_flags: HashMap<usize, bool>,
}

impl Context {
    pub fn init(&mut self, nodes: &Arc<[Node]>) {
        self.nodes = nodes.clone();
        // TODO: in-place clear
        self.values = vec![None; nodes.len()].into_boxed_slice();
        self.local_variables = Vec::with_capacity(8);
    }

    pub fn run_start(
        &mut self,
        nodes: Arc<[Node]>,
        idx: usize,
        values: Vec<Value>,
    ) {
        self.init(&nodes);

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
                    let mut stack = self.value_cache_get();

                    self.query_params(parameters, &mut stack);
                    COUNT.fetch_add(1, atomic::Ordering::SeqCst);
                    let branch_idx = exec.exec(self, &mut stack, 0);

                    if let Some(values) = &mut self.values[idx] {
                        values.clear();
                        values.extend_from_slice(&stack);
                        self.value_cache_ret(stack);
                    } else {
                        self.values[idx] = Some(stack)
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
        self.init(&nodes);

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

        let mut pending = self.pending_param_cache_get();
        pending
            .extend(params.iter().rev().copied().map(PendingParam::from));

        while let Some(PendingParam { idx, visited }) = pending.last_mut()
        {
            if *visited {
                let Node::Operation { parameters, exec } =
                    &nodes[idx.node]
                else {
                    panic!("expect Node::Operation");
                };

                let param_base = params_out.len() - parameters.len();

                COUNT.fetch_add(1, atomic::Ordering::SeqCst);
                exec.exec(self, params_out, param_base);
                params_out.swap(param_base, param_base + idx.value);
                params_out.truncate(param_base + 1);

                pending.pop();
            } else {
                match &nodes[idx.node] {
                    Node::Operation { parameters, exec } => {
                        if exec.manual_param() {
                            let output_base = params_out.len();

                            exec.exec_manual_param(
                                self, parameters, params_out,
                            );

                            params_out.swap(
                                output_base,
                                output_base + idx.value,
                            );
                            params_out.truncate(output_base + 1);

                            pending.pop();
                        } else {
                            *visited = true;
                            pending.extend(
                                parameters
                                    .iter()
                                    .rev()
                                    .copied()
                                    .map(PendingParam::from),
                            );
                        }
                    }

                    Node::Constant { values } => {
                        let v = values[idx.value].clone();
                        params_out.push(v);
                        pending.pop();
                    }
                    Node::Start { .. } | Node::Exec { .. } => {
                        let v = self.values[idx.node]
                            .as_ref()
                            .map(|it| it[idx.value].clone())
                            .unwrap_or_default();
                        params_out.push(v);
                        pending.pop();
                    }

                    Node::End { .. } => {
                        panic!("expect node that will output value")
                    }
                }
            }
        }

        self.pending_param_cache_ret(pending);
    }

    pub fn value_cache_get(&mut self) -> Vec<Value> {
        self.value_cache
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(8))
    }

    pub fn value_cache_ret(&mut self, mut value: Vec<Value>) {
        value.clear();
        self.value_cache.push(value);
    }

    pub fn pending_param_cache_get(&mut self) -> Vec<PendingParam> {
        self.pending_param_cache
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(8))
    }

    pub fn pending_param_cache_ret(
        &mut self,
        mut value: Vec<PendingParam>,
    ) {
        value.clear();
        self.pending_param_cache.push(value);
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
