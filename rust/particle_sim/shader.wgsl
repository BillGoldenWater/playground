struct Param {
    time_delta: f32,
    mouse_press: u32,
    mouse_pos: vec2<f32>,
    boundary_collision_factor: u32,
    global_velocity_damping: u32,
}

struct Point {
    pos: vec2<f32>,
    velocity: vec2<f32>,
}

struct PointHashToIdx {
    index: u32,
    hash: u32,
}

var<push_constant> param: Param;

@group(0)
@binding(0)
var<storage, read> points: array<Point>;

@group(0)
@binding(1)
var<storage, read_write> points_out: array<Point>;

@group(0)
@binding(2)
var<storage, read_write> points_hash_data: array<PointHashToIdx>;

@group(0)
@binding(3)
var<storage, read_write> points_hash_index: array<u32>;

struct VertexOut {
    @builtin(position)
    pos: vec4<f32>,
    @location(0)
    v_pos: vec2<f32>,
    @location(1)
    center: vec2<f32>,
    @location(2)
    color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @location(0) point_pos: vec2<f32>,
    @location(1) velocity: vec2<f32>,
) -> VertexOut {
    var vertices = array(
        vec2(-1.0, 1.0),
        vec2(-1.0, -1.0),
        vec2(1.0, -1.0),
        vec2(-1.0, 1.0),
        vec2(1.0, -1.0),
        vec2(1.0, 1.0),
    );

    let point_pos_clip = (point_pos * boundary_scaler - 0.5) * 2.0;
    let vertex_pos = vertices[in_vertex_index] * point_radius * sqrt(2.0) + point_pos_clip;

    let speed = length(velocity) / max_velocity_visual;
    let red = speed;
    let green = max(1 - speed, 0.0);
    let color = vec4<f32>(red, green, 0.0, 1.0) ;

    return VertexOut(
        vec4<f32>(vertex_pos, 0, 1),
        vertex_pos.xy,
        point_pos_clip,
        color,
    );
}

@fragment
fn fs_main(
    info: VertexOut,
) -> @location(0) vec4<f32> {
    let dst = distance(info.v_pos, info.center);
    let in_range = f32(dst < point_radius);

    let alpha = smoothstep(
        point_radius,
        point_radius - point_radius * edge_width,
        dst
    );

    return vec4<f32>(info.color.rgb, alpha * in_range);
}

// percent
const edge_width = 0.1;
const max_velocity_visual = 2000f;
const boundary_size = 80000.0;
const boundary_x = boundary_size;
const boundary_y = boundary_size; 
const grid_size = 300.0;
const point_size = 195.0;
const a = 200f;
const b = 50000f;
const gravity = 250.0;
const speed = 1.0;

const boundary_scaler = 1.0 / vec2<f32>(boundary_x, boundary_y);
const point_radius = point_size * boundary_scaler.x;
// const point_radius_squared = point_radius * point_radius;

const mouse_radius = boundary_size / 10f;
const mouse_strength = 20000f;

const gravity_center_count = 1u;
const gravity_centers = array<vec2<f32>, gravity_center_count>(
    vec2<f32>(boundary_x / 2, boundary_y / 2),
    // vec2<f32>(boundary_x / 2, boundary_y / 2 + 1250),
    // vec2<f32>(boundary_x / 2 - 1250, boundary_y / 2 - 1250),
    // vec2<f32>(boundary_x / 2 + 1250, boundary_y / 2 - 1250)
);

const grid_offset_count = 9u;
const grid_offsets = array<vec2<i32>, grid_offset_count>(
    vec2(-1, 1),
    vec2(-1, 0),
    vec2(-1, -1),
    vec2(0, 1),
    vec2(0, 0),
    vec2(0, -1),
    vec2(1, 1),
    vec2(1, 0),
    vec2(1, -1),
);

@compute
@workgroup_size(1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x + global_id.y * 65535 + global_id.z * 65535 * 65535;
    let time_delta = param.time_delta;

    var p = points[idx];

    p = update(idx, time_delta);

    // var limit = calc_limit(p);
    // let num_tick = limit.num_tick;
    // let tick_time_delta = param.time_delta / f32(num_tick);

    // for (var i = 0u; i < num_tick; i += 1u) {
    //     p.velocity *= limit.velocity_factor;

    //     p = update(idx, tick_time_delta);

    //     limit = calc_limit(p);
    // }

    points_out[idx] = p;
}

struct LimitInfo {
    num_tick: u32,
    velocity_factor: f32,
}

fn calc_limit(p: Point) -> LimitInfo {
    let tick_distance = length(p.velocity) * param.time_delta;
    let num_tick = max(u32(ceil(pow(tick_distance / 5f, 2f))), 1u);
    let num_tick_clamped = min(num_tick, 100u);

    return LimitInfo(
        num_tick_clamped,
        f32(num_tick_clamped) / f32(num_tick)
    );
}

fn update(idx: u32, time_delta: f32) -> Point {
    var p = points[idx];
    var pos = p.pos;
    // p.pos += p.velocity * 1f / 1000f;

    var acc = vec2<f32>(0.0, -gravity);

    // var gravity_centers = gravity_centers;
    // for (var i = 0u; i < gravity_center_count; i += 1u) {
    //     let center = gravity_centers[i];
    //     let to_center = normalize(center - p.pos);
    //     acc += to_center * gravity;
    // }

    let mouse_pos = param.mouse_pos * vec2(boundary_x, boundary_y);
    let to_mouse_distance_squared = distanse_squared(mouse_pos, p.pos);
    let mouse_in_range = to_mouse_distance_squared < pow(mouse_radius, 2f);
    if param.mouse_press > 0 && mouse_in_range {
        let to_mouse_distance = sqrt(to_mouse_distance_squared);
        var to_mouse = mouse_pos - p.pos;
        to_mouse = select(vec2(0f), to_mouse / to_mouse_distance, to_mouse_distance > 0.00001);

        let mouse_dir = (f32(param.mouse_press) - 1.5) * -2.0;
        var mouse_acc = mouse_dir * mouse_strength;

        let scaler = 1 - (to_mouse_distance / mouse_radius);
        acc += (to_mouse * mouse_acc - p.velocity) * scaler;
    }

    let grid_id = point_to_grid_id(p);
    var grid_offsets = grid_offsets;
    for (var offset_idx = 0u; offset_idx < grid_offset_count; offset_idx += 1u) {
        let id = grid_id + grid_offsets[offset_idx];
        let hash = grid_id_to_hash(id);
        let start_idx = points_hash_index[hash];

        for (var hash_idx = start_idx; hash_idx < arrayLength(&points_hash_data); hash_idx += 1u) {
            let point_hash = points_hash_data[hash_idx];
            if point_hash.hash != hash {
                break;
            }

            let point_idx = point_hash.index;
            let other_p = points[point_idx];

            if point_idx == idx || distance(other_p.pos, p.pos) > grid_size {
                continue;
            }

            let dst = distance(p.pos, other_p.pos);
            let force = b * (pow(a / dst, 12f) - pow(a / dst, 6f));

            let repel_direction = normalize(p.pos - other_p.pos);
            let accl = repel_direction * force;
            acc += accl;
        }
    }

    p.velocity += acc * time_delta;
    // p.velocity = acc;

    let x_out_up = (p.pos.x > boundary_x && p.velocity.x > 0);
    let y_out_up = (p.pos.y > boundary_y && p.velocity.y > 0);
    let x_out_bottom = (p.pos.x < 0 && p.velocity.x < 0);
    let y_out_bottom = (p.pos.y < 0 && p.velocity.y < 0);

    // collide box
    let collide_x = x_out_up || x_out_bottom;
    let collide_y = y_out_up || y_out_bottom;
    // let collide_x_pos = select(.0, boundary_x, x_out_up) + select(.0, .0, x_out_bottom);
    // let collide_y_pos = select(.0, boundary_y, y_out_up) + select(.0, .0, y_out_bottom);
    // p.pos.x = select(p.pos.x, collide_x_pos, collide_x);
    // p.pos.y = select(p.pos.y, collide_y_pos, collide_y);
    p.velocity.x *= select(1.0, -1.0, collide_x);
    p.velocity.y *= select(1.0, -1.0, collide_y);
    // p.velocity *= select(1.0, f32(param.boundary_collision_factor) * 0.01, collide_x || collide_y);
    p.velocity *= select(1.0, f32(param.boundary_collision_factor) * 0.01, y_out_bottom);

    // passthrough box
    // if x_out_up {
    //     p.pos.x = 0.0;
    // }
    // if x_out_bottom {
    //     p.pos.x = boundary_x;
    // }
    // if y_out_up {
    //     p.pos.y = 0.0;
    // }
    // if y_out_bottom {
    //     p.pos.y = boundary_y;
    // }

    p.velocity *= f32(param.global_velocity_damping) * 0.0001;

    p.pos = pos + p.velocity * time_delta;

    return p;
}

@compute
@workgroup_size(1)
fn calc_hash_data(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x + global_id.y * 65535 + global_id.z * 65535 * 65535;

    let grid_id = point_to_grid_id(points[idx]);
    let hash = grid_id_to_hash(grid_id);
    points_hash_data[idx] = PointHashToIdx(idx, hash);
}

@compute
@workgroup_size(1)
fn calc_hash_index(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x + global_id.y * 65535 + global_id.z * 65535 * 65535;

    let cur = points_hash_data[idx].hash;

    if idx == 0 {
        points_hash_index[cur] = idx;
    } else {
        let prev = points_hash_data[idx - 1].hash;
        if prev != cur {
            points_hash_index[cur] = idx;
        }
    }
}

fn point_to_grid_id(p: Point) -> vec2<i32> {
    return vec2<i32>(p.pos / grid_size);
}

fn grid_id_to_hash(id: vec2<i32>) -> u32 {
    let hash = u32(id.x) * 15823 + u32(id.y) + 9737333;
    return hash % arrayLength(&points_hash_data);
}

fn distanse_squared(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return pow(a.x - b.x, 2f) + pow(a.y - b.y, 2f);
}
