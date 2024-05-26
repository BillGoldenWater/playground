struct Data {
    value: u32,
}

@group(0) @binding(0) var<storage, read_write> data: array<Data>;

struct Param {
    dimension_size: u32,

    step: u32,
    op_len: u32,
}

var<push_constant> param: Param;

@compute
@workgroup_size(1)
fn bitonic_sort_op(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let y = global_id.y * param.dimension_size;
    let z = global_id.z * param.dimension_size * param.dimension_size;
    let op_id = global_id.x + y + z;

    let op_len = param.op_len;

    let op_offset_group = (op_id / op_len) * op_len * 2;
    let op_offset_op = op_id % op_len;
    let op_offset = op_offset_group + op_offset_op;

    let op_size_max = op_len * 2;

    let op_size_step_1 = (op_size_max - ((op_id * 2) % op_size_max)) - 1;
    let op_size = select(op_len, op_size_step_1, param.step == 1);

    let left = op_offset;
    let right = (op_offset + op_size);

    if right >= arrayLength(&data) {
        return;
    }

    let a = data[left];
    let b = data[right];

    let need_swap = a.value > b.value;
    if need_swap {
        let temp = data[left];
        data[left] = data[right];
        data[right] = temp;
    }
}
