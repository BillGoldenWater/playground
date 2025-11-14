use core::panic;
use std::{
    fmt::Debug,
    hash::Hash,
    ops::Index,
    sync::{
        Arc,
        atomic::{self, AtomicU32},
    },
    time::{Duration, Instant},
};

use crate::{
    logger::{Logger, Record, ValueSnapshot},
    value::Value,
};

pub static COUNT: AtomicU32 = AtomicU32::new(0);

pub mod logger;
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
        code: &Code,
        stack: &mut Vec<Value>,
        param_base: usize,
    ) -> usize;

    fn manual_param(&self) -> bool {
        false
    }

    /// fetch parameters and logging manually
    /// # Returns
    /// next branch index, ignored in non exec node
    fn exec_manual(
        &self,
        ctx: &mut Context,
        code: &Code,
        node: usize,
        params: &[ParameterIndexes],
        stack: &mut Vec<Value>,
    ) -> usize {
        let (..) = (ctx, code, node, params, stack);
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

#[derive(Debug)]
pub struct LogBegin {
    pub parameters: Box<[ValueSnapshot]>,
    pub start: Instant,
}

impl LogBegin {
    pub fn overwrite_parameters(
        begin: Option<&mut Self>,
        params: &[Value],
    ) {
        if let Some(begin) = begin {
            begin.parameters =
                ValueSnapshot::from_values_iter(params.iter().cloned())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Code<'code> {
    pub nodes: &'code [Node],
}

impl Index<usize> for Code<'_> {
    type Output = Node;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

#[derive(Debug, Default)]
pub struct Context {
    pub values: Box<[Option<Vec<Value>>]>,
    pub local_variables: Vec<Value>,
    pub value_cache: Vec<Vec<Value>>,
    pub pending_param_cache: Vec<Vec<PendingParam>>,

    pub logger: Option<Logger>,
    // TODO: , and clear
    // loop_flags: HashMap<usize, bool>,
}

impl Context {
    pub fn init(&mut self, code: &Code) {
        let nodes_len = code.nodes.len();
        if self.values.len() == nodes_len {
            self.values.fill(None);
        } else {
            self.values = vec![None; nodes_len].into_boxed_slice();
        }
        self.local_variables.clear();
        self.local_variables.reserve(8);
    }

    pub fn run_start(
        &mut self,
        code: &Code,
        idx: usize,
        values: Vec<Value>,
    ) {
        self.init(code);

        let Node::Start { next } = &code[idx] else {
            panic!("expect start node");
        };

        self.values[idx] = Some(values);

        for idx in next.iter().rev() {
            self.run_inner(code, idx.node);
        }
    }

    pub fn run_inner(&mut self, code: &Code, idx: usize) {
        let mut exec_queue = Vec::<usize>::with_capacity(8);
        exec_queue.push(idx);

        while let Some(idx) = exec_queue.pop() {
            match &code[idx] {
                Node::Exec {
                    parameters,
                    exec,
                    next,
                } => {
                    let mut stack = self.value_cache_get();

                    self.query_params(code, parameters, &mut stack);

                    COUNT.fetch_add(1, atomic::Ordering::SeqCst);
                    let log_begin = self.log_begin(&stack);
                    let branch_idx = exec.exec(self, code, &mut stack, 0);
                    self.log_end(log_begin, idx, &stack);

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

    pub fn run_end(&mut self, code: &Code, idx: usize) -> Box<[Value]> {
        self.init(code);

        let Node::End { parameters } = &code[idx] else {
            panic!("expect end node");
        };

        let mut output = self.value_cache_get();
        self.query_params(code, parameters, &mut output);
        output.into_boxed_slice()
    }

    pub fn query_params(
        &mut self,
        code: &Code,
        params: &[ParameterIndexes],
        params_out: &mut Vec<Value>,
    ) {
        let mut pending = self.pending_param_cache_get();
        pending
            .extend(params.iter().rev().copied().map(PendingParam::from));

        while let Some(PendingParam { idx, visited }) = pending.last_mut()
        {
            if *visited {
                let Node::Operation { parameters, exec } =
                    &code[idx.node]
                else {
                    panic!("expect Node::Operation");
                };

                let param_base = params_out.len() - parameters.len();

                COUNT.fetch_add(1, atomic::Ordering::SeqCst);
                let log_begin = self.log_begin(&params_out[param_base..]);
                exec.exec(self, code, params_out, param_base);
                self.log_end(
                    log_begin,
                    idx.node,
                    &params_out[param_base..],
                );

                params_out.swap(param_base, param_base + idx.value);
                params_out.truncate(param_base + 1);

                pending.pop();
            } else {
                match &code[idx.node] {
                    Node::Operation { parameters, exec } => {
                        if exec.manual_param() {
                            let output_base = params_out.len();

                            exec.exec_manual(
                                self, code, idx.node, parameters,
                                params_out,
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

    pub fn is_logging(&self) -> bool {
        self.logger.is_some()
    }

    pub fn log_begin_time(&self) -> Option<Instant> {
        self.is_logging().then(Instant::now)
    }

    pub fn log_begin(&mut self, params: &[Value]) -> Option<LogBegin> {
        if self.is_logging() {
            let parameters =
                ValueSnapshot::from_values_iter(params.iter().cloned());
            let start = Instant::now();
            Some(LogBegin { parameters, start })
        } else {
            None
        }
    }

    pub fn log_end(
        &mut self,
        begin: Option<LogBegin>,
        node: usize,
        outputs: &[Value],
    ) {
        if let Some(begin) = begin {
            self.log_end_subtract_duration(
                begin,
                node,
                outputs,
                Duration::ZERO,
            );
        }
    }

    pub fn log_end_subtract_duration(
        &mut self,
        begin: LogBegin,
        node: usize,
        outputs: &[Value],
        dur_sub: Duration,
    ) {
        let LogBegin { parameters, start } = begin;
        let duration = start.elapsed() - dur_sub;
        let outputs =
            ValueSnapshot::from_values_iter(outputs.iter().cloned());
        self.logger.as_mut().unwrap().record(Record {
            node,
            duration,
            parameters,
            outputs,
        });
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
