mod registers;
mod instructions;

use crate::cpu::instructions::{Instruction, Target};
use crate::cpu::registers::Registers;
use crate::mmu::Bus;
pub struct Cpu {
    registers: Registers,
    pc: u16,
    sp: u16,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
            pc: 0x0000,
            sp: 0x0000,
        }
    }
    
    pub fn step(&mut self, bus: &mut Bus) -> u32 {
        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        let instruction = Instruction::from_opcode(opcode, bus, &mut self.pc);
        
        let cycles = self.execute(instruction, bus);
        
        cycles
    }
    
    fn execute(&mut self, instruction: Instruction, bus: &mut Bus) -> u32 {
        match instruction {
            Instruction::NOP => 0,
            Instruction::LD(to, from) => 0,
            Instruction::INC(target) => 0,
            Instruction::DEC(target) => 0,
        }
    }
}