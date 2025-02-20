struct Data {
    value: u32,
}

@group(0) @binding(0) var<storage, read_write> data: array<Data>;

struct Param {
    dimension_size: u32,

    stage: u32,
    step: u32,
    step_log2: u32,
    step_mod_mask: u32,
}

var<push_constant> param: Param;

@compute
@workgroup_size(1)
fn bitonic_sort_op(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let y = global_id.y * param.dimension_size;
    let z = global_id.z * param.dimension_size * param.dimension_size;
    let op_id = global_id.x + y + z;

    let stage = param.stage;
    let step = param.step;
    let step_log2 = param.step_log2;

    let offset = (op_id >> step_log2) << (step_log2 + 1);
    let step_remainder = op_id & param.step_mod_mask;
    let left = offset + step_remainder;
    let right_first_step = offset + stage - 1 - step_remainder;
    let right_other = left + step;
    let right = select(right_other, right_first_step, stage >> 1 == step);

    if right >= arrayLength(&data) {
        return;
    }

    let a = data[left];
    let b = data[right];

    let need_swap = a.value > b.value;

    if need_swap {
        data[left] = b;
        data[right] = a;
    }
}
