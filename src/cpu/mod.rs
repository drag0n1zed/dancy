mod instructions;
mod registers;

use crate::cpu::instructions::{ByteDest, ByteLocation, ByteSource, JumpCondition, WordDest, WordSource};
use crate::cpu::registers::Registers;
use crate::mmu::Bus;

pub struct Cpu {
    pub registers: Registers,
    pub pc: u16,
    pub sp: u16,
    pub ime: bool,
    pub halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            registers: Registers::new(),
            pc: 0x0100,
            sp: 0xFFFE,
            ime: false,
            halted: false,
        }
    }

    pub async fn step(&mut self, bus: &mut Bus) {
        if self.halted {
            bus.tick().await;
            return;
        }

        let opcode = bus.read(self.pc).await;
        self.pc = self.pc.wrapping_add(1);

        match opcode {
            // NOP
            0x00 => {}

            // STOP
            0x10 => {
                let _ = bus.read(self.pc).await; // MUNCH
                self.pc = self.pc.wrapping_add(1);
                // todo: implement stop logic
            }

            // HALT
            0x76 => {
                self.halted = true;
                // todo: implement halt logic
            }

            // DI / EI
            0xF3 => {
                self.ime = false;
                // todo: DI
            }
            0xFB => {
                // todo: EI
            }

            // LD r, r'
            0x40..=0x7F => {
                let dest = self.decode_dest((opcode >> 3) & 0b111);
                let src = self.decode_source(opcode & 0b111);
                self.run_ld8(bus, dest, src).await;
            }
            // LD r, n
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
                let dest = self.decode_dest((opcode >> 3) & 0b111);
                let val = self.fetch_byte(bus).await;
                self.run_ld8(bus, dest, ByteSource::Immediate(val)).await;
            }
            // LD (HL), n
            0x36 => {
                let val = self.fetch_byte(bus).await;
                self.run_ld8(bus, ByteDest::IndHL, ByteSource::Immediate(val)).await;
            }
            // Indirect Loading A
            0x02 => self.run_ld8(bus, ByteDest::IndBC, ByteSource::A).await,
            0x12 => self.run_ld8(bus, ByteDest::IndDE, ByteSource::A).await,
            0x22 => self.run_ld8(bus, ByteDest::IndHLI, ByteSource::A).await,
            0x32 => self.run_ld8(bus, ByteDest::IndHLD, ByteSource::A).await,
            0x0A => self.run_ld8(bus, ByteDest::A, ByteSource::IndBC).await,
            0x1A => self.run_ld8(bus, ByteDest::A, ByteSource::IndDE).await,
            0x2A => self.run_ld8(bus, ByteDest::A, ByteSource::IndHLI).await,
            0x3A => self.run_ld8(bus, ByteDest::A, ByteSource::IndHLD).await,
            // LDH
            0xE0 => {
                // LD (0xFF00 + n), A
                let n = self.fetch_byte(bus).await;
                self.run_ld8(bus, ByteDest::Address(0xFF00 | n as u16), ByteSource::A)
                    .await;
            }
            0xF0 => {
                // LD A, (0xFF00 + n)
                let n = self.fetch_byte(bus).await;
                self.run_ld8(bus, ByteDest::A, ByteSource::Address(0xFF00 | n as u16))
                    .await;
            }
            0xE2 => self.run_ld8(bus, ByteDest::FF00PlusC, ByteSource::A).await,
            0xF2 => self.run_ld8(bus, ByteDest::A, ByteSource::FF00PlusC).await,
            // LD Absolute
            0xEA => {
                // LD (nn), A
                let addr = self.fetch_word(bus).await;
                self.run_ld8(bus, ByteDest::Address(addr), ByteSource::A).await;
            }
            0xFA => {
                // LD A, (nn)
                let addr = self.fetch_word(bus).await;
                self.run_ld8(bus, ByteDest::A, ByteSource::Address(addr)).await;
            }

            // LD rr, nn
            0x01 => {
                let w = self.fetch_word(bus).await;
                self.run_ld16(bus, WordDest::BC, WordSource::Immediate(w)).await;
            }
            0x11 => {
                let w = self.fetch_word(bus).await;
                self.run_ld16(bus, WordDest::DE, WordSource::Immediate(w)).await;
            }
            0x21 => {
                let w = self.fetch_word(bus).await;
                self.run_ld16(bus, WordDest::HL, WordSource::Immediate(w)).await;
            }
            0x31 => {
                let w = self.fetch_word(bus).await;
                self.run_ld16(bus, WordDest::SP, WordSource::Immediate(w)).await;
            }
            0xF9 => {
                // LD SP, HL
                bus.tick().await; // +1 cycle
                self.run_ld16(bus, WordDest::SP, WordSource::HL).await;
            }
            0x08 => {
                // LD (nn), SP
                let addr = self.fetch_word(bus).await;
                self.run_ld16(bus, WordDest::Address(addr), WordSource::SP).await;
            }

            // LDHL SP, n
            0xF8 => {
                let offset = self.fetch_byte(bus).await as i8;
                bus.tick().await; // +1 cycle
                self.run_ldhl(bus, offset).await;
            }

            // PUSH
            0xC5 => self.run_push(bus, WordSource::BC).await,
            0xD5 => self.run_push(bus, WordSource::DE).await,
            0xE5 => self.run_push(bus, WordSource::HL).await,
            0xF5 => self.run_push(bus, WordSource::AF).await,

            // POP
            0xC1 => self.run_pop(bus, WordDest::BC).await,
            0xD1 => self.run_pop(bus, WordDest::DE).await,
            0xE1 => self.run_pop(bus, WordDest::HL).await,
            0xF1 => self.run_pop(bus, WordDest::AF).await,

            // ALU
            // INC r
            0x04 | 0x14 | 0x24 | 0x34 | 0x0C | 0x1C | 0x2C | 0x3C => {
                let loc = self.decode_bits_to_location((opcode >> 3) & 0b111);
                self.run_inc8(bus, loc).await;
            }
            // DEC r
            0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => {
                let loc = self.decode_bits_to_location((opcode >> 3) & 0b111);
                self.run_dec8(bus, loc).await;
            }
            // ADD A, r
            0x80..=0x87 => {
                let src = self.decode_source(opcode & 0b111);
                self.run_add(bus, src).await;
            }
            // ADC A, r
            0x88..=0x8F => {
                let src = self.decode_source(opcode & 0b111);
                self.run_adc(bus, src).await;
            }
            // SUB A, r
            0x90..=0x97 => {
                let src = self.decode_source(opcode & 0b111);
                self.run_sub(bus, src).await;
            }
            // SBC A, r
            0x98..=0x9F => {
                let src = self.decode_source(opcode & 0b111);
                self.run_sbc(bus, src).await;
            }
            // AND A, r
            0xA0..=0xA7 => {
                let src = self.decode_source(opcode & 0b111);
                self.run_and(bus, src).await;
            }
            // XOR A, r
            0xA8..=0xAF => {
                let src = self.decode_source(opcode & 0b111);
                self.run_xor(bus, src).await;
            }
            // OR A, r
            0xB0..=0xB7 => {
                let src = self.decode_source(opcode & 0b111);
                self.run_or(bus, src).await;
            }
            // CP A, r
            0xB8..=0xBF => {
                let src = self.decode_source(opcode & 0b111);
                self.run_cp(bus, src).await;
            }

            // ALU Immediate
            0xC6 => {
                let v = self.fetch_byte(bus).await;
                self.run_add(bus, ByteSource::Immediate(v)).await;
            }
            0xCE => {
                let v = self.fetch_byte(bus).await;
                self.run_adc(bus, ByteSource::Immediate(v)).await;
            }
            0xD6 => {
                let v = self.fetch_byte(bus).await;
                self.run_sub(bus, ByteSource::Immediate(v)).await;
            }
            0xDE => {
                let v = self.fetch_byte(bus).await;
                self.run_sbc(bus, ByteSource::Immediate(v)).await;
            }
            0xE6 => {
                let v = self.fetch_byte(bus).await;
                self.run_and(bus, ByteSource::Immediate(v)).await;
            }
            0xEE => {
                let v = self.fetch_byte(bus).await;
                self.run_xor(bus, ByteSource::Immediate(v)).await;
            }
            0xF6 => {
                let v = self.fetch_byte(bus).await;
                self.run_or(bus, ByteSource::Immediate(v)).await;
            }
            0xFE => {
                let v = self.fetch_byte(bus).await;
                self.run_cp(bus, ByteSource::Immediate(v)).await;
            }

            // Other ALU
            0x27 => self.run_daa().await,
            0x37 => self.run_scf().await,
            0x2F => self.run_cpl().await,
            0x3F => self.run_ccf().await,

            // ALU16
            // INC16
            0x03 => self.run_inc16(bus, WordDest::BC).await,
            0x13 => self.run_inc16(bus, WordDest::DE).await,
            0x23 => self.run_inc16(bus, WordDest::HL).await,
            0x33 => self.run_inc16(bus, WordDest::SP).await,
            // DEC16
            0x0B => self.run_dec16(bus, WordDest::BC).await,
            0x1B => self.run_dec16(bus, WordDest::DE).await,
            0x2B => self.run_dec16(bus, WordDest::HL).await,
            0x3B => self.run_dec16(bus, WordDest::SP).await,
            // ADD HL, rr
            0x09 => self.run_addhl(bus, WordSource::BC).await,
            0x19 => self.run_addhl(bus, WordSource::DE).await,
            0x29 => self.run_addhl(bus, WordSource::HL).await,
            0x39 => self.run_addhl(bus, WordSource::SP).await,
            // ADD SP, n
            0xE8 => {
                let offset = self.fetch_byte(bus).await as i8;
                bus.tick().await; // +2 cycles
                bus.tick().await;
                self.run_addsp(bus, offset).await;
            }

            // CONTROL / BR
            // JR
            0x18 => self.run_jr(bus, JumpCondition::Always).await,
            0x20 => self.run_jr(bus, JumpCondition::NotZero).await,
            0x28 => self.run_jr(bus, JumpCondition::Zero).await,
            0x30 => self.run_jr(bus, JumpCondition::NoCarry).await,
            0x38 => self.run_jr(bus, JumpCondition::Carry).await,
            // JP
            0xC3 => self.run_jp(bus, JumpCondition::Always).await,
            0xC2 => self.run_jp(bus, JumpCondition::NotZero).await,
            0xCA => self.run_jp(bus, JumpCondition::Zero).await,
            0xD2 => self.run_jp(bus, JumpCondition::NoCarry).await,
            0xDA => self.run_jp(bus, JumpCondition::Carry).await,
            0xE9 => self.run_jp_hl(bus).await, // Special JP (HL)
            // CALL
            0xCD => self.run_call(bus, JumpCondition::Always).await,
            0xC4 => self.run_call(bus, JumpCondition::NotZero).await,
            0xCC => self.run_call(bus, JumpCondition::Zero).await,
            0xD4 => self.run_call(bus, JumpCondition::NoCarry).await,
            0xDC => self.run_call(bus, JumpCondition::Carry).await,
            // RET
            0xC9 => self.run_ret(bus, JumpCondition::Always).await,
            0xC0 => self.run_ret(bus, JumpCondition::NotZero).await,
            0xC8 => self.run_ret(bus, JumpCondition::Zero).await,
            0xD0 => self.run_ret(bus, JumpCondition::NoCarry).await,
            0xD8 => self.run_ret(bus, JumpCondition::Carry).await,
            0xD9 => self.run_reti(bus).await,

            // RST
            0xC7 | 0xD7 | 0xE7 | 0xF7 | 0xCF | 0xDF | 0xEF | 0xFF => {
                self.run_rst(bus, opcode & 0b0011_1000).await;
            }

            // CBs
            0xCB => {
                let cb_opcode = self.fetch_byte(bus).await; // +1 cycle
                let dest = self.decode_dest(cb_opcode & 0b0000_0111);
                let bit = (cb_opcode & 0b0011_1000) >> 3;
                match cb_opcode {
                    0x00..=0x07 => self.run_rlc(dest),
                    0x08..=0x0F => self.run_rrc(dest),
                    0x10..=0x17 => self.run_rl(dest),
                    0x18..=0x1F => self.run_rr(dest),
                    0x20..=0x27 => self.run_sla(dest),
                    0x28..=0x2F => self.run_sra(dest),
                    0x30..=0x37 => self.run_swap(dest),
                    0x38..=0x3F => self.run_srl(dest),
                    0x40..=0x7F => self.run_bit(dest, bit),
                    0x80..=0xBF => self.run_res(dest, bit),
                    0xC0..=0xFF => self.run_set(dest, bit),
                }
            }

            // RLCA
            0x07 => todo!("RLA"),
            // RLA
            0x17 => todo!("RLA"),
            // RRCA
            0x0F => todo!("RRCA"),
            // RRA
            0x1F => todo!("RRA"),

            0xD3 | 0xE3 | 0xE4 | 0xF4 | 0xDB | 0xEB | 0xEC | 0xFC | 0xDD | 0xED | 0xFD => {
                panic!("Illegal opcode: {:#04X}", opcode);
            }
        }
    }

    // Fetchers
    async fn fetch_byte(&mut self, bus: &mut Bus) -> u8 {
        let b = bus.read(self.pc).await;
        self.pc = self.pc.wrapping_add(1);
        b
    }

    async fn fetch_word(&mut self, bus: &mut Bus) -> u16 {
        let lo = self.fetch_byte(bus).await;
        let hi = self.fetch_byte(bus).await;
        u16::from_le_bytes([lo, hi])
    }

    // Logic
    async fn run_jr(&mut self, bus: &mut Bus, cond: JumpCondition) {
        // i8 offset
        let value = self.fetch_byte(bus).await as i8;
        if self.jump_condition_reached(cond) {
            bus.tick().await; // +1 cycle
            self.pc = self.pc.wrapping_add_signed(value as i16);
        }
    }

    async fn run_jp(&mut self, bus: &mut Bus, cond: JumpCondition) {
        let value = self.fetch_word(bus).await;
        if self.jump_condition_reached(cond) {
            bus.tick().await; // +1 cycle
            self.pc = value;
        }
    }

    async fn run_jp_hl(&mut self, _bus: &mut Bus) {
        // +0 cycles
        let value = self.registers.get_hl();
        self.pc = value;
    }

    async fn run_ret(&mut self, bus: &mut Bus, cond: JumpCondition) {
        bus.tick().await; // +1 cycle
        if self.jump_condition_reached(cond) {
            if let JumpCondition::Always = cond {
            } else {
                bus.tick().await; // +1 cycle if conditional match
            }
            let value = bus.read_u16(self.sp).await;
            self.sp = self.sp.wrapping_add(2);
            self.pc = value;
        }
    }

    async fn run_reti(&mut self, bus: &mut Bus) {
        let value = bus.read_u16(self.sp).await;
        self.sp = self.sp.wrapping_add(2);
        bus.tick().await; // +1 cycle
        self.pc = value;
        self.ime = true;
    }

    async fn run_call(&mut self, bus: &mut Bus, cond: JumpCondition) {
        let value = self.fetch_word(bus).await;
        if self.jump_condition_reached(cond) {
            bus.tick().await; // +1 cycle
            self.sp = self.sp.wrapping_sub(2);
            bus.write_u16(self.sp, self.pc).await; // +2 cycles
            self.pc = value;
        }
    }

    async fn run_rst(&mut self, bus: &mut Bus, lsb: u8) {
        bus.tick().await; // +1 cycle
        self.sp = self.sp.wrapping_sub(2);
        bus.write_u16(self.sp, self.pc).await; // +2 cycles
        let msb = 0x00;
        self.pc = u16::from_le_bytes([lsb, msb]);
    }

    async fn run_ld8(&mut self, bus: &mut Bus, dest: ByteDest, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        self.write_byte_dest(bus, dest, value).await;
    }

    async fn run_ld16(&mut self, bus: &mut Bus, dest: WordDest, source: WordSource) {
        let value = self.resolve_word_source(bus, source).await;
        self.write_word_dest(bus, dest, value).await;
    }

    async fn run_ldhl(&mut self, bus: &mut Bus, val: i8) {
        let val_unsigned = val as u8;

        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (self.sp & 0x000F) + (val_unsigned & 0x0F) as u16 > 0x000F;
        self.registers.f.carry = (self.sp & 0x00FF) + val_unsigned as u16 > 0xFF;

        let sp_plus_val = self.sp.wrapping_add_signed(val as i16);
        self.write_word_dest(bus, WordDest::HL, sp_plus_val).await;
    }

    async fn run_push(&mut self, bus: &mut Bus, source: WordSource) {
        let value = self.resolve_word_source(bus, source).await;
        bus.tick().await; // Internal cycle
        self.sp = self.sp.wrapping_sub(2);
        bus.write_u16(self.sp, value).await;
    }

    async fn run_pop(&mut self, bus: &mut Bus, dest: WordDest) {
        let value = bus.read_u16(self.sp).await;
        self.sp = self.sp.wrapping_add(2);
        self.write_word_dest(bus, dest, value).await;
    }

    async fn run_inc8(&mut self, bus: &mut Bus, loc: ByteLocation) {
        let val = self.resolve_byte_source(bus, loc.into()).await;
        let new_val = val.wrapping_add(1);

        self.registers.f.zero = new_val == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (val & 0x0F) == 0x0F;

        self.write_byte_dest(bus, loc.into(), new_val).await;
    }

    async fn run_dec8(&mut self, bus: &mut Bus, loc: ByteLocation) {
        let val = self.resolve_byte_source(bus, loc.into()).await;
        let new_val = val.wrapping_sub(1);

        self.registers.f.zero = new_val == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (val & 0x0F) == 0x00;

        self.write_byte_dest(bus, loc.into(), new_val).await;
    }

    async fn run_inc16(&mut self, bus: &mut Bus, loc: WordDest) {
        let value = self.resolve_word_source(bus, self.word_dest_to_src(loc)).await;
        bus.tick().await; // 16-bit ALU internal cycle
        self.write_word_dest(bus, loc, value.wrapping_add(1)).await;
    }

    async fn run_dec16(&mut self, bus: &mut Bus, loc: WordDest) {
        let value = self.resolve_word_source(bus, self.word_dest_to_src(loc)).await;
        bus.tick().await; // 16-bit ALU internal cycle
        self.write_word_dest(bus, loc, value.wrapping_sub(1)).await;
    }

    async fn run_addhl(&mut self, bus: &mut Bus, source: WordSource) {
        let value = self.resolve_word_source(bus, source).await;
        bus.tick().await; // Internal cycle
        let hl = self.registers.get_hl();
        let (new_hl, carry) = hl.overflowing_add(value);

        self.registers.f.subtract = false;
        self.registers.f.half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
        self.registers.f.carry = carry;

        self.registers.set_hl(new_hl);
    }

    async fn run_addsp(&mut self, _bus: &mut Bus, val: i8) {
        let val_unsigned = val as u8;

        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (self.sp & 0x000F) + (val_unsigned & 0x0F) as u16 > 0x000F;
        self.registers.f.carry = (self.sp & 0x00FF) + val_unsigned as u16 > 0xFF;

        self.sp = self.sp.wrapping_add_signed(val as i16);
    }

    async fn run_add(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let (new_a, carry) = a.overflowing_add(value);

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (a & 0x0F) + (value & 0x0F) > 0x0F;
        self.registers.f.carry = carry;

        self.registers.a = new_a;
    }

    async fn run_adc(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let c = if self.registers.f.carry { 1 } else { 0 };
        let new_word_a = (a as u16) + (value as u16) + (c as u16);
        let new_byte_a = new_word_a as u8;

        self.registers.f.zero = new_byte_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (a & 0x0F) + (value & 0x0F) + c > 0x0F;
        self.registers.f.carry = new_word_a > 0xFF;

        self.registers.a = new_byte_a;
    }

    async fn run_sub(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let (new_a, carry) = a.overflowing_sub(value);

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (a & 0x0F) < (value & 0x0F);
        self.registers.f.carry = carry;

        self.registers.a = new_a;
    }

    async fn run_sbc(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let c = if self.registers.f.carry { 1 } else { 0 };
        let new_word_a = (a as i16) - (value as i16) - c;
        let new_byte_a = new_word_a as u8;

        self.registers.f.zero = new_byte_a == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (a & 0x0F) as i16 - (value & 0x0F) as i16 - c < 0;
        self.registers.f.carry = new_word_a < 0;

        self.registers.a = new_byte_a;
    }

    async fn run_and(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let new_a = a & value;

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = true;
        self.registers.f.carry = false;

        self.registers.a = new_a;
    }

    async fn run_xor(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let new_a = a ^ value;

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;

        self.registers.a = new_a;
    }

    async fn run_or(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let new_a = a | value;

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;

        self.registers.a = new_a;
    }

    async fn run_cp(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let (new_a, carry) = a.overflowing_sub(value);

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (a & 0x0F) < (value & 0x0F);
        self.registers.f.carry = carry;
    }

    async fn run_daa(&mut self) {
        let a = self.registers.a;
        let mut adjust = 0x00;
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
    }

    async fn run_scf(&mut self) {
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = true;
    }

    async fn run_cpl(&mut self) {
        let a = self.registers.a;
        let new_a = !a;

        self.registers.f.subtract = true;
        self.registers.f.half_carry = true;

        self.registers.a = new_a;
    }

    async fn run_ccf(&mut self) {
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = !self.registers.f.carry;
    }

    // r/w
    async fn resolve_byte_source(&mut self, bus: &mut Bus, source: ByteSource) -> u8 {
        match source {
            ByteSource::A => self.registers.a,
            ByteSource::B => self.registers.b,
            ByteSource::C => self.registers.c,
            ByteSource::D => self.registers.d,
            ByteSource::E => self.registers.e,
            ByteSource::H => self.registers.h,
            ByteSource::L => self.registers.l,

            ByteSource::IndBC => bus.read(self.registers.get_bc()).await,
            ByteSource::IndDE => bus.read(self.registers.get_de()).await,
            ByteSource::IndHL => bus.read(self.registers.get_hl()).await,
            ByteSource::IndHLI => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_add(1));
                bus.read(addr).await
            }
            ByteSource::IndHLD => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_sub(1));
                bus.read(addr).await
            }
            ByteSource::FF00PlusC => bus.read(u16::from_le_bytes([self.registers.c, 0xFF])).await,
            ByteSource::Address(word) => bus.read(word).await,
            ByteSource::Immediate(byte) => byte,
        }
    }

    async fn write_byte_dest(&mut self, bus: &mut Bus, dest: ByteDest, value: u8) {
        match dest {
            ByteDest::A => self.registers.a = value,
            ByteDest::B => self.registers.b = value,
            ByteDest::C => self.registers.c = value,
            ByteDest::D => self.registers.d = value,
            ByteDest::E => self.registers.e = value,
            ByteDest::H => self.registers.h = value,
            ByteDest::L => self.registers.l = value,

            ByteDest::IndBC => bus.write(self.registers.get_bc(), value).await,
            ByteDest::IndDE => bus.write(self.registers.get_de(), value).await,
            ByteDest::IndHL => bus.write(self.registers.get_hl(), value).await,
            ByteDest::IndHLI => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_add(1));
                bus.write(addr, value).await
            }
            ByteDest::IndHLD => {
                let addr = self.registers.get_hl();
                self.registers.set_hl(addr.wrapping_sub(1));
                bus.write(addr, value).await
            }
            ByteDest::FF00PlusC => bus.write(0xFF00 | self.registers.c as u16, value).await,
            ByteDest::Address(word) => bus.write(word, value).await,
        }
    }

    async fn resolve_word_source(&mut self, _bus: &mut Bus, source: WordSource) -> u16 {
        match source {
            WordSource::AF => self.registers.get_af(),
            WordSource::BC => self.registers.get_bc(),
            WordSource::DE => self.registers.get_de(),
            WordSource::HL => self.registers.get_hl(),
            WordSource::SP => self.sp,
            WordSource::Immediate(word) => word,
        }
    }

    async fn write_word_dest(&mut self, bus: &mut Bus, dest: WordDest, value: u16) {
        match dest {
            WordDest::AF => self.registers.set_af(value),
            WordDest::BC => self.registers.set_bc(value),
            WordDest::DE => self.registers.set_de(value),
            WordDest::HL => self.registers.set_hl(value),
            WordDest::SP => self.sp = value,
            WordDest::Address(word) => {
                bus.write_u16(word, value).await;
            }
        }
    }

    fn jump_condition_reached(&self, cond: JumpCondition) -> bool {
        match cond {
            JumpCondition::NotZero => !self.registers.f.zero,
            JumpCondition::Zero => self.registers.f.zero,
            JumpCondition::NoCarry => !self.registers.f.carry,
            JumpCondition::Carry => self.registers.f.carry,
            JumpCondition::Always => true,
        }
    }

    // opcode decode
    fn decode_dest(&self, bits: u8) -> ByteDest {
        self.decode_bits_to_location(bits).into()
    }

    fn decode_source(&self, bits: u8) -> ByteSource {
        self.decode_bits_to_location(bits).into()
    }

    fn decode_bits_to_location(&self, bits: u8) -> ByteLocation {
        match bits {
            0b000 => ByteLocation::B,
            0b001 => ByteLocation::C,
            0b010 => ByteLocation::D,
            0b011 => ByteLocation::E,
            0b100 => ByteLocation::H,
            0b101 => ByteLocation::L,
            0b110 => ByteLocation::IndHL,
            0b111 => ByteLocation::A,
            _ => unreachable!(),
        }
    }

    // Helper to map INC16/DEC16 Dest to Source logic (since it reads then writes)
    fn word_dest_to_src(&self, dest: WordDest) -> WordSource {
        match dest {
            WordDest::BC => WordSource::BC,
            WordDest::DE => WordSource::DE,
            WordDest::HL => WordSource::HL,
            WordDest::SP => WordSource::SP,
            _ => unreachable!(),
        }
    }
}
