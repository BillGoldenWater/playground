use bytemuck::NoUninit;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, NoUninit)]
#[repr(C)]
pub struct Param {
    pub time_delta: f32,
    pub mouse_press: u32,
    pub mouse_pos: [f32; 2],
    pub boundary_collision_factor: u32,
    pub global_velocity_damping: u32,
}

impl Default for Param {
    fn default() -> Self {
        Self {
            time_delta: 1f32 / 1000f32,
            mouse_press: 0,
            mouse_pos: [0.0, 0.0],
            boundary_collision_factor: 100,
            global_velocity_damping: 10000,
        }
    }
}
