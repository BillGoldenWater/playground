#[derive(Debug, Clone, Copy)]
pub enum InterpreterState {
    Paused,
    Running,
}

impl InterpreterState {
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}
