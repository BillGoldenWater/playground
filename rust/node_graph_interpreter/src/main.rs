use std::{
    env::args,
    sync::atomic,
    time::{Duration, Instant},
};

use anyhow::Context as _;
use node_graph_interpreter::{
    COUNT, Code, Context, FlowIndexes, Node, ParameterIndexes,
    logger::Logger,
    nodes::{
        ADDITION, DOUBLE_BRANCH, FINITE_LOOP, IS_GREATER_THAN,
        LIST_ASSEMBLE, LIST_GET, LIST_LENGTH, LIST_SET, LOCAL_VARIABLE,
        LOCAL_VARIABLE_SET, SUBTRACTION,
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
    let param_n = |node, value| ParameterIndexes { node, value };
    let param = |node| param_n(node, 0);
    let flow = |node| FlowIndexes { node };

    let nodes = &[
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
    ];

    // avoid any possible compile time optimization
    // for this specific nodes combination
    let nodes = core::hint::black_box(nodes);
    let code = Code { nodes };

    let mut ctx = Context::default();

    let run_dur = 1.;

    let mut count = 0;
    let mut cost_sum = Duration::default();
    let mut min = Duration::MAX;
    let mut max = Duration::default();
    while cost_sum.as_secs_f64() < run_dur {
        let start = Instant::now();

        ctx.run_start(&code, 1, [].into());

        let dur = start.elapsed();
        cost_sum += dur;
        min = dur.min(min);
        max = dur.max(max);
        count += 1;
    }
    println!(
        "run count: {count}, avg: {:?}, min: {min:?}, max: {max:?} - node graph bubble sort",
        cost_sum / count
    );
    println!(
        "node run: {}",
        COUNT.load(atomic::Ordering::SeqCst) / count
    );
    println!("{:?}", ctx.local_variables[0]);
    // println!("{:?}", ctx.value_cache);
    // println!("{:?}", ctx.pending_param_cache);

    let Some(arg) = args().nth(1) else {
        return Ok(());
    };
    let flags = arg.parse::<u64>().context("parsing arg")?;

    if flags & 0b10 != 0 {
        let mut ctx = Context {
            logger: Some(Logger::default()),
            ..Context::default()
        };
        ctx.run_start(&code, 1, [].into());
        ctx.logger.as_mut().unwrap().print_per_node(&code);
    }

    if flags & 1 == 0 {
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
        "run count: {count}, avg: {:?}, min: {min:?}, max: {max:?} - naive bubble sort",
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
        "run count: {count}, avg: {:?}, min: {min:?}, max: {max:?} - std lib sort",
        cost_sum / count
    );
    println!("{arr:?}");

    Ok(())
}
