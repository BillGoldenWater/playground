use std::{sync::Arc, time::Duration};

use crate::{Code, value::Value};

#[derive(Debug, Default)]
pub struct Logger {
    pub logs: Vec<Record>,
}

impl Logger {
    pub fn clear(&mut self) {
        self.logs.clear();
    }

    pub fn record(&mut self, record: Record) {
        self.logs.push(record);
    }

    pub fn to_per_node(
        &self,
        code: &Code,
    ) -> Box<[Option<RecordPerNode<'_>>]> {
        #[derive(Debug, Clone)]
        struct RecordPerNodeWorking<'log> {
            total_duration: Duration,
            runs: Vec<RunParametersAndOutputs<'log>>,
        }

        let mut result =
            vec![Option::<RecordPerNodeWorking>::None; code.nodes.len()];

        for rec in &self.logs {
            let run = RunParametersAndOutputs {
                parameters: &rec.parameters,
                outputs: &rec.outputs,
            };
            if let Some(node) = &mut result[rec.node] {
                node.total_duration += rec.duration;
                node.runs.push(run);
            } else {
                result[rec.node] = Some(RecordPerNodeWorking {
                    total_duration: rec.duration,
                    runs: vec![run],
                });
            }
        }

        result
            .into_iter()
            .map(|it| {
                it.map(|it| RecordPerNode {
                    total_duration: it.total_duration,
                    runs: it.runs.into_boxed_slice(),
                })
            })
            .collect()
    }

    pub fn print_per_node(&self, code: &Code) {
        let per_node = self.to_per_node(code);
        let len = per_node.len();
        for (idx, node) in per_node.into_iter().enumerate() {
            println!("node {idx}:");
            let node = if let Some(node) = node {
                node
            } else {
                println!("  None");
                continue;
            };
            println!("  total_duration: {:?}", node.total_duration);
            println!("  run_count: {}", node.runs.len());
            for (idx, run) in node.runs.into_iter().enumerate() {
                println!("  run {idx}:");
                if !run.parameters.is_empty() {
                    if run.parameters.len() > 1 {
                        println!("    in:");
                        for v in run.parameters {
                            println!("      {v:?}")
                        }
                    } else {
                        println!("    in: {:?}", run.parameters[0]);
                    }
                }
                if !run.outputs.is_empty() {
                    if run.outputs.len() > 1 {
                        println!("    out:");
                        for v in run.outputs {
                            println!("      {v:?}")
                        }
                    } else {
                        println!("    out: {:?}", run.outputs[0]);
                    }
                }
            }
            if idx != len - 1 {
                println!();
            }
        }
    }
}

#[derive(Debug)]
pub struct RecordPerNode<'log> {
    pub total_duration: Duration,
    pub runs: Box<[RunParametersAndOutputs<'log>]>,
}

#[derive(Debug, Clone, Copy)]
pub struct RunParametersAndOutputs<'log> {
    pub parameters: &'log [ValueSnapshot],
    pub outputs: &'log [ValueSnapshot],
}

#[derive(Debug)]
pub struct Record {
    pub node: usize,
    pub duration: Duration,
    pub parameters: Box<[ValueSnapshot]>,
    pub outputs: Box<[ValueSnapshot]>,
}

#[derive(Debug, Clone, Default)]
pub enum ValueSnapshot {
    #[default]
    Uninit,
    None,

    Bool(bool),
    Int(i64),

    String(Arc<str>),
    List(Box<[ValueSnapshot]>),

    LoopId(usize),
    LocalVariable(usize),
}

impl ValueSnapshot {
    pub fn from_values_iter(
        values: impl IntoIterator<Item = Value>,
    ) -> Box<[Self]> {
        values.into_iter().map(Self::from_value).collect()
    }

    pub fn from_value(value: Value) -> Self {
        match value {
            Value::Uninit => Self::Uninit,
            Value::None => Self::None,
            Value::Bool(v) => Self::Bool(v),
            Value::Int(v) => Self::Int(v),
            Value::String(v) => Self::String(v),
            Value::List(v) => Self::List(
                v.borrow()
                    .iter()
                    .cloned()
                    .map(Self::from_value)
                    .collect(),
            ),
            Value::LoopId(id) => Self::LoopId(id),
            Value::LocalVariable(key) => Self::LocalVariable(key),
        }
    }
}
