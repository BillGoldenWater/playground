use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use node_graph_interpreter::{
    Code, Context, FlowIndexes, Node, ParameterIndexes,
    logger::Logger,
    nodes::{
        ADDITION, DOUBLE_BRANCH, FINITE_LOOP, IS_GREATER_THAN,
        LIST_ASSEMBLE, LIST_GET, LIST_LENGTH, LIST_SET, LOCAL_VARIABLE,
        LOCAL_VARIABLE_SET, SUBTRACTION,
    },
    value::Value,
};

fn bubble_sort(c: &mut Criterion) {
    let nodes = nodes();
    let nodes = core::hint::black_box(&nodes);
    let code = Code { nodes };

    let mut ctx = Context::default();

    let mut group = c.benchmark_group("bubble_sort/node_graph");

    group.bench_function("normal", |b| {
        b.iter(|| {
            ctx.run_start(&code, 1, [].into());
            std::hint::black_box(&ctx);
        })
    });

    ctx.logger = Some(Logger::default());
    group.bench_function("logged", |b| {
        b.iter_custom(|iters| {
            let mut dur = Duration::default();
            for _ in 0..iters {
                ctx.logger.as_mut().unwrap().clear();
                let start = Instant::now();
                ctx.run_start(&code, 1, [].into());
                std::hint::black_box(&ctx);
                dur += start.elapsed();
            }
            dur
        })
    });
    group.finish();

    c.bench_function("bubble_sort_naive", |b| {
        b.iter(|| {
            let mut arr = std::hint::black_box(vec![2, 1, 4, 6, 0]);
            for _ in 0..arr.len() {
                for i in 0..arr.len() - 1 {
                    if arr[i] > arr[i + 1] {
                        arr.swap(i, i + 1);
                    }
                }
            }
            arr
        })
    });

    c.bench_function("std_sort", |b| {
        b.iter(|| {
            let mut arr = std::hint::black_box(vec![2, 1, 4, 6, 0]);
            arr.sort();
            arr
        })
    });
}

criterion_group!(benches, bubble_sort);
criterion_main!(benches);

fn nodes() -> Box<[Node]> {
    let constant = |value| ParameterIndexes { node: 0, value };
    let param_n = |node, value| ParameterIndexes { node, value };
    let param = |node| param_n(node, 0);
    let flow = |node| FlowIndexes { node };

    [
        // 0
        Node::Constant {
            values: [
                // 0: list[0]
                Value::Int(2),
                // 1: list[1]
                Value::Int(1),
                // 2: list[2]
                Value::Int(4),
                // 3: list[3]
                Value::Int(6),
                // 4: list[4]
                Value::Int(0),
                // 5: list
                Value::LocalVariable(0),
                // 6: list len - 1
                Value::LocalVariable(1),
                // 7: temp
                Value::LocalVariable(2),
                // 8: 0
                Value::Int(0),
                // 9: 1
                Value::Int(1),
            ]
            .into(),
        },
        // 1
        Node::Start {
            next: [flow(7)].into(),
        },
        // 2 assemble list
        Node::Operation {
            parameters: [
                constant(0),
                constant(1),
                constant(2),
                constant(3),
                constant(4),
            ]
            .into(),
            exec: LIST_ASSEMBLE,
        },
        // 3 local variable, list
        Node::Operation {
            parameters: [constant(5), param(2)].into(),
            exec: LOCAL_VARIABLE,
        },
        // 4 list length
        Node::Operation {
            parameters: [param(3)].into(),
            exec: LIST_LENGTH,
        },
        // 5 list length - 1
        Node::Operation {
            parameters: [param(4), constant(9)].into(),
            exec: SUBTRACTION,
        },
        // 6 local variable, list length - 1
        Node::Operation {
            parameters: [constant(6), param(5)].into(),
            exec: LOCAL_VARIABLE,
        },
        // 7 loop 1, 0..=(len - 1)
        Node::Exec {
            parameters: [constant(8), param(6)].into(),
            next: [[flow(9)].into(), [].into()].into(),
            exec: FINITE_LOOP,
        },
        // 8 list length - 2
        Node::Operation {
            parameters: [param(6), constant(9)].into(),
            exec: SUBTRACTION,
        },
        // 9 loop 2, 0..=(len - 2)
        Node::Exec {
            parameters: [constant(8), param(8)].into(),
            next: [[flow(15)].into(), [].into()].into(),
            exec: FINITE_LOOP,
        },
        // 10 loop 2 idx + 1
        Node::Operation {
            parameters: [param(9), constant(9)].into(),
            exec: ADDITION,
        },
        // 11 list[loop 2 idx]
        Node::Operation {
            parameters: [param(3), param(9)].into(),
            exec: LIST_GET,
        },
        // 12 list[loop 2 idx + 1]
        Node::Operation {
            parameters: [param(3), param(10)].into(),
            exec: LIST_GET,
        },
        // 13 list[loop 2 idx] > list[loop 2 idx + 1]
        Node::Operation {
            parameters: [param(11), param(12)].into(),
            exec: IS_GREATER_THAN,
        },
        // 14 list[loop 2 idx] > list[loop 2 idx + 1]
        Node::Operation {
            parameters: [param(11), param(12)].into(),
            exec: IS_GREATER_THAN,
        },
        // 15 if list[loop 2 idx] > list[loop 2 idx + 1]
        Node::Exec {
            parameters: [param(14)].into(),
            next: [[flow(16)].into(), [].into()].into(),
            exec: DOUBLE_BRANCH,
        },
        // 16 set temp = list[loop 2 idx]
        Node::Exec {
            parameters: [constant(7), param(11)].into(),
            next: [[flow(17)].into()].into(),
            exec: LOCAL_VARIABLE_SET,
        },
        // 17 set list[loop 2 idx] = list[loop 2 idx + 1]
        Node::Exec {
            parameters: [param(3), param(9), param(12)].into(),
            next: [[flow(19)].into()].into(),
            exec: LIST_SET,
        },
        // 18 local variable temp
        Node::Operation {
            parameters: [constant(7)].into(),
            exec: LOCAL_VARIABLE,
        },
        // 19 set list[loop 2 idx + 1] = temp
        Node::Exec {
            parameters: [param(3), param(10), param(18)].into(),
            next: [[].into()].into(),
            exec: LIST_SET,
        },
    ]
    .into()
}
