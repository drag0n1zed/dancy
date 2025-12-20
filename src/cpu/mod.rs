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
            Instruction::LDHL(val) => {
                let val_unsigned = val as u8;

                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.sp & 0x000F) + (val_unsigned & 0x0F) as u16 > 0x000F;
                self.registers.f.carry = (self.sp & 0x00FF) + val_unsigned as u16 > 0xFF;

                let sp_plus_val = self.sp.wrapping_add_signed(val as i16);
                self.write_word_dest(bus, WordDest::HL, sp_plus_val);
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
            Instruction::INC8(loc) => {
                let val = self.resolve_byte_source(bus, loc.into());
                let new_val = val.wrapping_add(1);

                self.registers.f.zero = new_val == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (val & 0x0F) == 0x0F; // Overflow if 0bxxxx1111

                self.write_byte_dest(bus, loc.into(), new_val);
                false
            }
            Instruction::DEC8(loc) => {
                let val = self.resolve_byte_source(bus, loc.into());
                let new_val = val.wrapping_sub(1);

                self.registers.f.zero = new_val == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (val & 0x0F) == 0x00; // Overflow if 0bxxxx0000

                self.write_byte_dest(bus, loc.into(), new_val);
                false
            }
            Instruction::INC16(loc) => {
                let value = self.resolve_word_source(loc.into());
                self.write_word_dest(bus, loc.into(), value.wrapping_add(1));
                // Doesn't affect flags
                false
            }
            Instruction::DEC16(loc) => {
                let value = self.resolve_word_source(loc.into());
                self.write_word_dest(bus, loc.into(), value.wrapping_sub(1));
                // Doesn't affect flags
                false
            }
            Instruction::ADDHL(source) => {
                let value = self.resolve_word_source(source);
                let hl = self.registers.get_hl();
                let (new_hl, carry) = hl.overflowing_add(value);

                self.registers.f.subtract = false;
                self.registers.f.half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
                self.registers.f.carry = carry;

                self.registers.set_hl(new_hl);
                false
            }
            Instruction::ADDSP(val) => {
                let val_unsigned = val as u8;

                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.sp & 0x000F) + (val_unsigned & 0x0F) as u16 > 0x000F;
                self.registers.f.carry = (self.sp & 0x00FF) + val_unsigned as u16 > 0xFF;

                self.sp = self.sp.wrapping_add_signed(val as i16);
                false
            }
            Instruction::ADD(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let (new_a, carry) = a.overflowing_add(value);

                self.registers.f.zero = new_a == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (a & 0x0F) + (value & 0x0F) > 0x0F;
                self.registers.f.carry = carry;

                self.registers.a = new_a;
                false
            }
            Instruction::ADC(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let c = if self.registers.f.carry { 1 } else { 0 };
                let new_word_a = (a as u16) + (value as u16) + (c as u16);
                let new_byte_a = new_word_a as u8;

                self.registers.f.zero = new_byte_a == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry =  (a & 0x0F) + (value & 0x0F) + c > 0x0F;
                self.registers.f.carry = new_word_a > 0xFF;

                self.registers.a = new_byte_a;
                false
            }
            Instruction::SUB(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let (new_a, carry) = a.overflowing_sub(value);

                self.registers.f.zero = new_a == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (a & 0x0F) < (value & 0x0F);
                self.registers.f.carry = carry;

                self.registers.a = new_a;
                false
            }
            Instruction::SBC(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let c = if self.registers.f.carry { 1 } else { 0 };
                let new_word_a = (a as i16) - (value as i16) - c;
                let new_byte_a = new_word_a as u8;

                self.registers.f.zero = new_byte_a == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry =  (a & 0x0F) as i16 - (value & 0x0F) as i16 - c < 0;
                self.registers.f.carry = new_word_a < 0;

                self.registers.a = new_byte_a;
                false
            }
            Instruction::AND(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let new_a = a & value;

                self.registers.f.zero = new_a == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = true;
                self.registers.f.carry = false;

                self.registers.a = new_a;
                false
            }
            Instruction::XOR(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let new_a = a ^ value;

                self.registers.f.zero = new_a == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = false;

                self.registers.a = new_a;
                false
            }
            Instruction::OR(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let new_a = a | value;

                self.registers.f.zero = new_a == 0;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = false;

                self.registers.a = new_a;
                false
            }
            Instruction::CP(source) => {
                let value = self.resolve_byte_source(bus, source);
                let a = self.registers.a;
                let (new_a, carry) = a.overflowing_sub(value);

                self.registers.f.zero = new_a == 0;
                self.registers.f.subtract = true;
                self.registers.f.half_carry = (a & 0x0F) < (value & 0x0F);
                self.registers.f.carry = carry;

                // Does not update register
                false
            }
            Instruction::DAA => {
                let a = self.registers.a;
                let mut adjust = 0x00; // 0x00, 0x06, 0x60 or 0x66
                let mut carry = false;
                if self.registers.f.half_carry || (!self.registers.f.subtract && (a & 0x0F) > 0x09) {
                    adjust |= 0x06;
                }
                if self.registers.f.carry || (!self.registers.f.subtract && a > 0x99) {
                    adjust |= 0x60;
                    carry = true;
                }
                let new_a = match self.registers.f.subtract {
                    true => a.wrapping_sub(adjust),
                    false => a.wrapping_add(adjust),
                };

                self.registers.f.zero = new_a == 0;
                self.registers.f.half_carry = false;
                self.registers.f.carry = carry;

                self.registers.a = new_a;
                false
            }
            Instruction::SCF => {
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = true;

                false
            }
            Instruction::CPL => {
                let a = self.registers.a;
                let new_a = !a;

                self.registers.f.subtract = true;
                self.registers.f.half_carry = true;

                self.registers.a = new_a;
                false
            }
            Instruction::CCF => {
                self.registers.f.subtract = false;
                self.registers.f.half_carry = false;
                self.registers.f.carry = !self.registers.f.carry;

                false
            }

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
