mod instructions;
mod registers;

use crate::cpu::instructions::{ByteDest, ByteSource, Instruction, WordDest, WordSource};
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
            pc: 0x0100,
            sp: 0xFFFE,
            ime: false,
            ime_scheduled: false,
            halted: false,
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> u32 {
        if self.halted {
            return 4;
        }

        let opcode = bus.read(self.pc);
        let (instruction, op_bytes, (base_cycles, cond_met_cycles)) = Instruction::from_opcode(opcode, bus, self.pc);
        self.pc = self.pc.wrapping_add(op_bytes);

        let cond = self.execute(instruction, bus);
        // RETURN T-CYCLES!
        if cond { cond_met_cycles } else { base_cycles }
    }

    fn execute(&mut self, instruction: Instruction, bus: &mut Bus) -> bool {
        // Did condition get matched?
        match instruction {
            // control/misc
            Instruction::NOP => false,

            // control/br

            // lsm
            Instruction::LD8(dest, source) => {
                let value = self.resolve_byte_source(bus, source);
                self.write_byte_dest(bus, dest, value);
                false
            }
            Instruction::LD16(dest, source) => {
                let value = self.resolve_word_source(source);
                self.write_word_dest(bus, dest, value);
                false
            }
            Instruction::PUSH(source) => {
                let value = self.resolve_word_source(source); // Value in register
                self.sp = self.sp.wrapping_sub(2); // Simulate SP decrement
                bus.write_u16(self.sp, value); // Write value in register into stack
                false
            }
            Instruction::POP(dest) => {
                let value = bus.read_u16(self.sp);
                self.sp = self.sp.wrapping_add(2);
                self.write_word_dest(bus, dest, value);
                false
            }
            // alu

            // rsb
            _ => unimplemented!(),
        }
    }

    // Could modify registers.
    fn resolve_byte_source(&mut self, bus: &Bus, source: ByteSource) -> u8 {
        match source {
            ByteSource::A => self.registers.a,
            ByteSource::B => self.registers.b,
            ByteSource::C => self.registers.c,
            ByteSource::D => self.registers.d,
            ByteSource::E => self.registers.e,
            ByteSource::H => self.registers.h,
            ByteSource::L => self.registers.l,

            ByteSource::IndBC => bus.read(self.registers.get_bc()),
            ByteSource::IndDE => bus.read(self.registers.get_de()),
            ByteSource::IndHL => bus.read(self.registers.get_hl()),
            ByteSource::IndHLI => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_add(1));
                bus.read(addr)
            }
            ByteSource::IndHLD => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_sub(1));
                bus.read(addr)
            }

            ByteSource::FF00PlusC => bus.read(0xFF00 | self.registers.c as u16),

            ByteSource::Address(word) => bus.read(word),
            ByteSource::Immediate(byte) => byte,
        }
    }

    fn write_byte_dest(&mut self, bus: &mut Bus, dest: ByteDest, value: u8) {
        match dest {
            ByteDest::A => self.registers.a = value,
            ByteDest::B => self.registers.b = value,
            ByteDest::C => self.registers.c = value,
            ByteDest::D => self.registers.d = value,
            ByteDest::E => self.registers.e = value,
            ByteDest::H => self.registers.h = value,
            ByteDest::L => self.registers.l = value,

            ByteDest::IndBC => bus.write(self.registers.get_bc(), value),
            ByteDest::IndDE => bus.write(self.registers.get_de(), value),
            ByteDest::IndHL => bus.write(self.registers.get_hl(), value),
            ByteDest::IndHLI => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_add(1));
                bus.write(addr, value)
            }
            ByteDest::IndHLD => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_sub(1));
                bus.write(addr, value)
            }

            ByteDest::FF00PlusC => bus.write(0xFF00 | self.registers.c as u16, value),

            ByteDest::Address(word) => bus.write(word, value),
        }
    }

    fn resolve_word_source(&mut self, source: WordSource) -> u16 {
        match source {
            WordSource::AF => self.registers.get_af(),
            WordSource::BC => self.registers.get_bc(),
            WordSource::DE => self.registers.get_de(),
            WordSource::HL => self.registers.get_hl(),

            WordSource::SP => self.sp,

            WordSource::Immediate(word) => word,

            WordSource::SPPlusImmediate(byte) => self.sp.wrapping_add_signed(byte as i16),
        }
    }

    fn write_word_dest(&mut self, bus: &mut Bus, dest: WordDest, value: u16) {
        match dest {
            WordDest::AF => self.registers.set_af(value),
            WordDest::BC => self.registers.set_bc(value),
            WordDest::DE => self.registers.set_de(value),
            WordDest::HL => self.registers.set_hl(value),

            WordDest::SP => self.sp = value,

            WordDest::Address(word) => {
                bus.write_u16(word, value);
            }
        }
    }
}
