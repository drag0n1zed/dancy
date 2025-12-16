mod instructions;
mod registers;

use crate::cpu::instructions::{Instruction};
use crate::cpu::registers::Registers;
use crate::mmu::Bus;
pub struct Cpu {
    registers: Registers,
    pc: u16,
    sp: u16,
    ime: bool,
    ime_scheduled: bool, 
    halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
            pc: 0x0000,
            sp: 0x0000,
            ime: true,
            ime_scheduled: false,
            halted: false,
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> u32 {
        if self.halted {
            return 4;
        }

        let opcode = bus.read(self.pc);

        let (instruction, op_bytes) = Instruction::from_opcode(opcode, bus, self.pc);
        self.pc = self.pc.wrapping_add(op_bytes);

        self.execute(instruction, bus)
        // RETURN T-CYCLES!
    }

    fn execute(&mut self, instruction: Instruction, bus: &mut Bus) -> u32 {
        match instruction {
            // control/misc
            Instruction::NOP => 4,
            _ => unimplemented!(),
        }
    }
}
