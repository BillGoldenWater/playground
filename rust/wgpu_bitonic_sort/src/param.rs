#[derive(Debug, Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
pub struct Param {
    pub dimension_size: u32,

    pub stage: u32,
    pub step: u32,
    pub step_log2: u32,
    pub step_mod_mask: u32,
}
