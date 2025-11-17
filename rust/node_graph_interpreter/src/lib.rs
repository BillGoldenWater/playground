use core::panic;
use std::{
    fmt::Debug,
    hash::Hash,
    ops::Index,
    time::{Duration, Instant},
};

use crate::{
    logger::{Logger, Record, ValueSnapshot},
    value::Value,
    vec_pool::VecPool,
};

pub mod logger;
pub mod nodes;
pub mod value;
pub mod vec_pool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParameterIndexes {
    pub node: usize,
    pub value: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlowIndexes {
    pub node: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum Exec {
    Default(
        fn(
            ctx: &mut Context,
            code: &Code,
            stack: &mut Vec<Value>,
            param_base: usize,
        ) -> usize,
    ),
    Manual(
        fn(
            ctx: &mut Context,
            code: &Code,
            node: usize,
            params: &[ParameterIndexes],
            stack: &mut Vec<Value>,
        ) -> usize,
    ),
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

        exec: Exec,
    },
    Operation {
        parameters: Box<[ParameterIndexes]>,

        exec: Exec,
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
    pub logger: Option<Logger>,

    pub values: Box<[Option<Vec<Value>>]>,
    pub local_variables: Vec<Value>,
    pub loop_flags: Vec<bool>,

    pub pool_usize: VecPool<usize>,
    pub pool_value: VecPool<Value>,
    pub pool_pending_param: VecPool<PendingParam>,
}

impl Context {
    pub fn init(&mut self, code: &Code) {
        let nodes_len = code.nodes.len();
        if self.values.len() == nodes_len {
            self.values.fill(None);
        } else {
            self.values = vec![None; nodes_len].into_boxed_slice();
        }

        self.loop_flags.clear();

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
        let mut exec_queue = self.pool_usize.get();
        exec_queue.push(idx);

        while let Some(idx) = exec_queue.pop() {
            match &code[idx] {
                Node::Exec {
                    parameters,
                    exec,
                    next,
                } => {
                    let mut stack = self.pool_value.get();

                    let branch_idx = match exec {
                        Exec::Default(exec) => {
                            self.query_params(
                                code, parameters, &mut stack,
                            );

                            let log_begin = self.log_begin(&stack);
                            let branch_idx =
                                exec(self, code, &mut stack, 0);
                            self.log_end(log_begin, idx, &stack);

                            branch_idx
                        }
                        Exec::Manual(exec) => {
                            exec(self, code, idx, parameters, &mut stack)
                        }
                    };

                    if let Some(values) = &mut self.values[idx] {
                        values.clear();
                        values.extend_from_slice(&stack);
                        self.pool_value.ret(stack);
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

        self.pool_usize.ret(exec_queue);
    }

    pub fn run_end(&mut self, code: &Code, idx: usize) -> Box<[Value]> {
        self.init(code);

        let Node::End { parameters } = &code[idx] else {
            panic!("expect end node");
        };

        let mut output = self.pool_value.get();
        self.query_params(code, parameters, &mut output);
        output.into_boxed_slice()
    }

    pub fn query_params(
        &mut self,
        code: &Code,
        params: &[ParameterIndexes],
        params_out: &mut Vec<Value>,
    ) {
        let mut pending = self.pool_pending_param.get();
        for it in params.iter().rev() {
            pending.push(PendingParam::from(*it));
        }

        while let Some(PendingParam { idx, visited }) = pending.last_mut()
        {
            if *visited {
                let Node::Operation { parameters, exec } =
                    &code[idx.node]
                else {
                    panic!("expect Node::Operation");
                };

                let param_base = params_out.len() - parameters.len();

                let log_begin = self.log_begin(&params_out[param_base..]);
                let Exec::Default(exec) = exec else {
                    unreachable!(
                        "expect only Default will be marked visited"
                    );
                };
                exec(self, code, params_out, param_base);
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
                    Node::Operation { parameters, exec } => match exec {
                        Exec::Default(_) => {
                            *visited = true;
                            for it in parameters.iter().rev() {
                                pending.push(PendingParam::from(*it));
                            }
                        }
                        Exec::Manual(exec) => {
                            let output_base = params_out.len();

                            exec(
                                self, code, idx.node, parameters,
                                params_out,
                            );

                            params_out.swap(
                                output_base,
                                output_base + idx.value,
                            );
                            params_out.truncate(output_base + 1);

                            pending.pop();
                        }
                    },

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

        self.pool_pending_param.ret(pending);
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

    pub fn get_local_variable(&mut self, key: usize) -> &mut Value {
        if key >= self.local_variables.len() {
            self.local_variables.resize(key + 1, Value::Uninit);
        }

        &mut self.local_variables[key]
    }

    pub fn loop_enter(&mut self) -> usize {
        self.loop_flags.push(true);
        self.loop_flags.len() - 1
    }

    pub fn loop_is_running(&self, id: usize) -> bool {
        self.loop_flags[id]
    }

    pub fn loop_break(&mut self, id: usize) {
        self.loop_flags[id] = false;
    }

    pub fn loop_exit(&mut self, id: usize) {
        self.loop_flags.truncate(id);
    }
}
