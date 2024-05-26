#[derive(Debug, Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
pub struct Param {
    pub dimension_size: u32,
    pub step: u32,
    pub op_len: u32,
}
