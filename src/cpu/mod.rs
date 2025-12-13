mod instructions;
mod registers;

use crate::cpu::instructions::Instruction;
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

        let instruction = Instruction::from_opcode(opcode, bus, &mut self.pc);

        self.execute(instruction, bus)
        // RETURN T-CYCLES!
    }

    fn execute(&mut self, instruction: Instruction, bus: &mut Bus) -> u32 {
        match instruction {
            
            Instruction::NOP => {
                self.pc = self.pc.wrapping_add(1);
                4
            },
            _ => todo!(),
        }
    }

    fn check_and_service_interrupt(&mut self, bus: &mut Bus) -> bool {
        let ie_reg = bus.read(0xFFFF); // (Is this) Interrupt Enable(d?) Flag
        let if_reg = bus.read(0xFF0F); // (Did this) Interrupt (trigger?) Flag

        // Pending if same bit is set for both registers
        let pending = ie_reg & if_reg;

        // No interrupts
        if pending == 0 {
            return false;
        }
        todo!()
    }
}
