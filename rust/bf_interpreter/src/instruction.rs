#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    PtrInc,
    PtrDec,
    Inc,
    Dec,
    Prt,
    Read,
    JmpNext(usize),
    JmpPrev(usize),
}
