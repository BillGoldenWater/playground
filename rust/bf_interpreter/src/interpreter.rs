use std::{collections::VecDeque, path::Path, str::FromStr};

use anyhow::{bail, Context};
use functional_utils::FunctionalUtils;

use crate::instruction::Instruction;

#[derive(Debug, Default)]
pub struct Interpreter {
    pub instructions: Vec<Instruction>,

    pub input_buf: VecDeque<u8>,
    pub waitting_input: bool,
    pub output: Vec<u8>,

    pub memory: Vec<u8>,
    pub memory_ptr: usize,
    pub instruction_ptr: usize,
}

impl FromStr for Interpreter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut this = Self::default();

        let mut lines = s.lines();

        while let Some(line) = lines.next() {
            if line == "/end" {
                break;
            }

            line.chars()
                .into_iter()
                .filter_map(|it| match it {
                    '>' => Instruction::PtrInc.some(),
                    '<' => Instruction::PtrDec.some(),
                    '+' => Instruction::Inc.some(),
                    '-' => Instruction::Dec.some(),
                    '.' => Instruction::Prt.some(),
                    ',' => Instruction::Read.some(),
                    '[' => Instruction::JmpNext(0).some(),
                    ']' => Instruction::JmpPrev(0).some(),
                    _ => None,
                })
                .then(|it| this.instructions.extend(it));
        }

        if this.instructions.is_empty() {
            bail!("empty instructions");
        }
        this.parse_semantics()?;

        let rest = lines.collect::<Vec<&str>>();

        this.input_buf.extend(rest.join("\n").into_bytes());

        Ok(this)
    }
}

impl Interpreter {
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::read_to_string(path).context("failed to read file")?;

        Self::from_str(&file)
    }

    fn parse_semantics(&mut self) -> anyhow::Result<()> {
        let mut unmatched_jump = Vec::<usize>::new();

        for idx in 0..self.instructions.len() {
            match self.instructions[idx] {
                Instruction::JmpNext(_) => {
                    unmatched_jump.push(idx);
                }
                Instruction::JmpPrev(_) => {
                    let addr = if let Some(addr) = unmatched_jump.pop() {
                        addr
                    } else {
                        bail!("unmatched ]")
                    };

                    match &mut self.instructions[addr] {
                        Instruction::JmpNext(to) => *to = idx + 1,
                        _ => {
                            unreachable!("expect JmpNext")
                        }
                    }

                    match &mut self.instructions[idx] {
                        Instruction::JmpPrev(to) => *to = addr + 1,
                        _ => {
                            unreachable!("expect JmpPrev")
                        }
                    }
                }
                _ => {}
            }
        }

        if !unmatched_jump.is_empty() {
            bail!("unmatched [");
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        *self = Self {
            instructions: self.instructions.clone(),
            ..Default::default()
        }
    }

    pub fn tick(&mut self) -> bool {
        if self.instruction_ptr >= self.instructions.len() {
            return true;
        }

        if self.memory.is_empty() {
            self.memory.push(0);
        }

        let Self {
            instructions,
            memory,
            memory_ptr,
            instruction_ptr,
            input_buf,
            waitting_input,
            output,
        } = self;

        let instruction = instructions[*instruction_ptr];

        let mut new_instruction_ptr = *instruction_ptr + 1;

        match instruction {
            Instruction::PtrInc => {
                *memory_ptr += 1;
                if memory.len() == *memory_ptr {
                    memory.push(0);
                }
            }
            Instruction::PtrDec => {
                *memory_ptr -= 1;
            }
            Instruction::Inc => {
                memory[*memory_ptr] = memory[*memory_ptr].wrapping_add(1);
            }
            Instruction::Dec => memory[*memory_ptr] = memory[*memory_ptr].wrapping_sub(1),
            Instruction::Prt => {
                output.push(memory[*memory_ptr]);
            }
            Instruction::Read => {
                if let Some(value) = input_buf.pop_front() {
                    *waitting_input = false;
                    memory[*memory_ptr] = value;
                } else {
                    *waitting_input = true;
                    return false;
                }
            }
            Instruction::JmpNext(addr) => {
                if memory[*memory_ptr] == 0 {
                    new_instruction_ptr = addr
                }
            }
            Instruction::JmpPrev(addr) => {
                if memory[*memory_ptr] != 0 {
                    new_instruction_ptr = addr
                }
            }
        }

        *instruction_ptr = new_instruction_ptr;

        false
    }
}
