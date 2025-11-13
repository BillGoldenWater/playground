use std::{
    env::args,
    sync::Arc,
    time::{Duration, Instant},
};

use node_graph_interpreter::{
    Context, FlowIndexes, Node, ParameterIndexes,
    nodes::{
        Addition, DoubleBranch, IsGreaterThan, IsLessThan, ListAssemble,
        ListGet, ListLength, ListSet, LocalVariable, LocalVariableSet,
        Noop, Subtraction,
    },
    value::Value,
};
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let constant = |value| ParameterIndexes { node: 0, value };
    let param = |node, value| ParameterIndexes { node, value };
    let flow = |node| FlowIndexes { node };

    let b = Arc::from(DoubleBranch);
    let var = Arc::from(LocalVariable);
    let var_set = Arc::from(LocalVariableSet);
    let list_get = Arc::from(ListGet);
    let list_set = Arc::from(ListSet);
    let lt = Arc::from(IsLessThan);
    let gt = Arc::from(IsGreaterThan);
    let sub = Arc::from(Subtraction);
    let add = Arc::from(Addition);

    let nodes: Arc<[Node]> = vec![
        // 0
        Node::Constant {
            values: [
                // 0: len
                Value::LocalVariable(0),
                // 1: test
                Value::String("test".into()),
                // 2: loop 1 index
                Value::LocalVariable(1),
                // 3: 0
                Value::Int(0),
                // 4: 1
                Value::Int(1),
                // 5: list
                Value::LocalVariable(2),
                // 6: list[0]
                Value::Int(2),
                // 7: list[1]
                Value::Int(1),
                // 8: list[2]
                Value::Int(4),
                // 9: list[3]
                Value::Int(6),
                // 10: list[4]
                Value::Int(0),
                // 11: loop 2 index
                Value::LocalVariable(3),
                // 12: temp
                Value::LocalVariable(4),
            ]
            .into(),
        },
        // 1
        Node::Exec {
            parameters: [].into(),
            next: [].into(),
            exec: Arc::from(Noop),
        },
        // 2
        Node::Start {
            next: [flow(3)].into(),
        },
        // 3 set len = list len
        Node::Exec {
            parameters: [constant(0), param(11, 0)].into(),
            next: [[flow(4)].into()].into(),
            exec: var_set.clone(),
        },
        // 4 if loop 1 index < len
        Node::Exec {
            parameters: [param(5, 0)].into(),
            next: [[flow(8)].into(), [].into()].into(),
            exec: b.clone(),
        },
        // 5 loop 1 index < len
        Node::Operation {
            parameters: [param(6, 0), param(7, 0)].into(),
            exec: lt.clone(),
        },
        // 6 loop 1 index or 0
        Node::Operation {
            parameters: [constant(2), constant(3)].into(),
            exec: var.clone(),
        },
        // 7 len
        Node::Operation {
            parameters: [constant(0)].into(),
            exec: var.clone(),
        },
        // 8 set loop 2 index = 0
        Node::Exec {
            parameters: [constant(11), constant(3)].into(),
            next: [[flow(14)].into()].into(),
            exec: var_set.clone(),
        },
        // 9 loop 1 index++
        Node::Exec {
            parameters: [constant(2), param(10, 0)].into(),
            next: [[flow(4)].into()].into(),
            exec: var_set.clone(),
        },
        // 10 loop 1 index + 1
        Node::Operation {
            parameters: [param(6, 0), constant(4)].into(),
            exec: add.clone(),
        },
        // 11 list len
        Node::Operation {
            parameters: [param(12, 0)].into(),
            exec: Arc::from(ListLength),
        },
        // 12 list or assemble
        Node::Operation {
            parameters: [constant(5), param(13, 0)].into(),
            exec: var.clone(),
        },
        // 13 assemble
        Node::Operation {
            parameters: [
                constant(6),
                constant(7),
                constant(8),
                constant(9),
                constant(10),
            ]
            .into(),
            exec: Arc::from(ListAssemble),
        },
        // 14 if loop 2 index < len - 1
        Node::Exec {
            parameters: [param(15, 0)].into(),
            next: [[flow(18)].into(), [flow(9)].into()].into(),
            exec: b.clone(),
        },
        // 15 loop 2 index < len - 1
        Node::Operation {
            parameters: [param(16, 0), param(17, 0)].into(),
            exec: lt.clone(),
        },
        // 16 loop 2 index
        Node::Operation {
            parameters: [constant(11)].into(),
            exec: var.clone(),
        },
        // 17 len - 1
        Node::Operation {
            parameters: [param(7, 0), constant(4)].into(),
            exec: sub.clone(),
        },
        // 18 if list[loop 2 index] > list[loop 2 index + 1]
        Node::Exec {
            parameters: [param(21, 0)].into(),
            next: [[flow(25)].into(), [flow(19)].into()].into(),
            exec: b.clone(),
        },
        // 19 loop 2 index++
        Node::Exec {
            parameters: [constant(11), param(20, 0)].into(),
            next: [[flow(14)].into()].into(),
            exec: var_set.clone(),
        },
        // 20 loop 2 index + 1
        Node::Operation {
            parameters: [param(16, 0), constant(4)].into(),
            exec: add.clone(),
        },
        // 21 list[loop 2 index] > list[loop 2 index + 1]
        Node::Operation {
            parameters: [param(22, 0), param(23, 0)].into(),
            exec: gt.clone(),
        },
        // 22 list[loop 2 index]
        Node::Operation {
            parameters: [param(12, 0), param(16, 0)].into(),
            exec: list_get.clone(),
        },
        // 23 list[loop 2 index + 1]
        Node::Operation {
            parameters: [param(12, 0), param(24, 0)].into(),
            exec: list_get.clone(),
        },
        // 24 loop 2 index + 1
        Node::Operation {
            parameters: [param(16, 0), constant(4)].into(),
            exec: add.clone(),
        },
        // 25 swap, temp = list[loop 2 index]
        Node::Exec {
            parameters: [constant(12), param(22, 0)].into(),
            next: [[flow(26)].into()].into(),
            exec: var_set.clone(),
        },
        // 26 swap, list[loop 2 index] = list[loop 2 index + 1]
        Node::Exec {
            parameters: [param(12, 0), param(16, 0), param(23, 0)].into(),
            next: [[flow(27)].into()].into(),
            exec: list_set.clone(),
        },
        // 27 swap, list[loop 2 index + 1] = temp
        Node::Exec {
            parameters: [param(12, 0), param(20, 0), param(28, 0)].into(),
            next: [[flow(19)].into()].into(),
            exec: list_set.clone(),
        },
        // 28 temp
        Node::Operation {
            parameters: [constant(12)].into(),
            exec: var.clone(),
        },
    ]
    .into();

    // avoid any possible compile time optimization
    // for this specific nodes combination
    let nodes = core::hint::black_box(nodes);

    let mut ctx = Context::default();

    let run_dur = 1.;

    let mut count = 0;
    let mut cost_sum = Duration::default();
    let mut min = Duration::MAX;
    let mut max = Duration::default();
    while cost_sum.as_secs_f64() < run_dur {
        let nodes = nodes.clone();
        let start = Instant::now();

        ctx.run_start(nodes, 2, [].into());

        let dur = start.elapsed();
        cost_sum += dur;
        min = dur.min(min);
        max = dur.max(max);
        count += 1;
    }
    println!(
        "avg: {:?}, min: {min:?}, max: {max:?} - node graph bubble sort",
        cost_sum / count
    );
    println!("{:?}", ctx.local_variables[2]);

    if args().nth(1).is_none() {
        return Ok(());
    }

    let mut count = 0;
    let mut cost_sum = Duration::from_secs_f64(0.0);
    let mut arr = vec![];
    let mut min = Duration::MAX;
    let mut max = Duration::default();
    while cost_sum.as_secs_f64() < run_dur {
        let start = Instant::now();

        arr = std::hint::black_box(vec![2, 1, 4, 6, 0]);
        for _ in 0..arr.len() {
            for i in 0..arr.len() - 1 {
                if arr[i] > arr[i + 1] {
                    arr.swap(i, i + 1);
                }
            }
        }

        let dur = start.elapsed();
        cost_sum += dur;
        min = dur.min(min);
        max = dur.max(max);
        count += 1;
        std::hint::black_box(&arr);
    }
    println!(
        "avg: {:?}, min: {min:?}, max: {max:?} - bubble sort",
        cost_sum / count
    );
    println!("{arr:?}");

    let mut count = 0;
    let mut cost_sum = Duration::from_secs_f64(0.0);
    let mut arr = vec![];
    let mut min = Duration::MAX;
    let mut max = Duration::default();
    while cost_sum.as_secs_f64() < run_dur {
        let start = Instant::now();

        arr = std::hint::black_box(vec![2, 1, 4, 6, 0]);
        arr.sort();

        let dur = start.elapsed();
        cost_sum += dur;
        min = dur.min(min);
        max = dur.max(max);
        count += 1;
        std::hint::black_box(&arr);
    }
    println!(
        "avg: {:?}, min: {min:?}, max: {max:?} - std lib sort",
        cost_sum / count
    );
    println!("{arr:?}");

    Ok(())
}
