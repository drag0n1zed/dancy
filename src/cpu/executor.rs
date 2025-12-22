use crate::cpu::Cpu;
use crate::cpu::opcodes::{ByteDest, ByteLocation, ByteSource, JumpCondition, WordDest, WordSource};
use crate::mmu::Bus;

impl Cpu {
    // Fetchers
    pub(super) async fn fetch_byte(&mut self, bus: &mut Bus) -> u8 {
        let b = bus.read(self.pc).await;
        self.pc = self.pc.wrapping_add(1);
        b
    }

    pub(super) async fn fetch_word(&mut self, bus: &mut Bus) -> u16 {
        let lo = self.fetch_byte(bus).await;
        let hi = self.fetch_byte(bus).await;
        u16::from_le_bytes([lo, hi])
    }

    // Logic
    pub(super) async fn run_jr(&mut self, bus: &mut Bus, cond: JumpCondition) {
        // i8 offset
        let value = self.fetch_byte(bus).await as i8;
        if self.jump_condition_reached(cond) {
            bus.tick().await; // +1 cycle
            self.pc = self.pc.wrapping_add_signed(value as i16);
        }
    }

    pub(super) async fn run_jp(&mut self, bus: &mut Bus, cond: JumpCondition) {
        let value = self.fetch_word(bus).await;
        if self.jump_condition_reached(cond) {
            bus.tick().await; // +1 cycle
            self.pc = value;
        }
    }

    pub(super) async fn run_jp_hl(&mut self, _bus: &mut Bus) {
        // +0 cycles
        let value = self.registers.get_hl();
        self.pc = value;
    }

    pub(super) async fn run_ret(&mut self, bus: &mut Bus, cond: JumpCondition) {
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

    pub(super) async fn run_reti(&mut self, bus: &mut Bus) {
        let value = bus.read_u16(self.sp).await;
        self.sp = self.sp.wrapping_add(2);
        bus.tick().await; // +1 cycle
        self.pc = value;
        self.ime = true;
    }

    pub(super) async fn run_call(&mut self, bus: &mut Bus, cond: JumpCondition) {
        let value = self.fetch_word(bus).await;
        if self.jump_condition_reached(cond) {
            bus.tick().await; // +1 cycle
            self.sp = self.sp.wrapping_sub(2);
            bus.write_u16(self.sp, self.pc).await; // +2 cycles
            self.pc = value;
        }
    }

    pub(super) async fn run_rst(&mut self, bus: &mut Bus, lsb: u8) {
        bus.tick().await; // +1 cycle
        self.sp = self.sp.wrapping_sub(2);
        bus.write_u16(self.sp, self.pc).await; // +2 cycles
        let msb = 0x00;
        self.pc = u16::from_le_bytes([lsb, msb]);
    }

    pub(super) async fn run_ld8(&mut self, bus: &mut Bus, dest: ByteDest, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        self.write_byte_dest(bus, dest, value).await;
    }

    pub(super) async fn run_ld16(&mut self, bus: &mut Bus, dest: WordDest, source: WordSource) {
        let value = self.resolve_word_source(bus, source).await;
        self.write_word_dest(bus, dest, value).await;
    }

    pub(super) async fn run_ldhl(&mut self, bus: &mut Bus, val: i8) {
        let val_unsigned = val as u8;

        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (self.sp & 0x000F) + (val_unsigned & 0x0F) as u16 > 0x000F;
        self.registers.f.carry = (self.sp & 0x00FF) + val_unsigned as u16 > 0xFF;

        let sp_plus_val = self.sp.wrapping_add_signed(val as i16);
        self.write_word_dest(bus, WordDest::HL, sp_plus_val).await;
    }

    pub(super) async fn run_push(&mut self, bus: &mut Bus, source: WordSource) {
        let value = self.resolve_word_source(bus, source).await;
        bus.tick().await; // Internal cycle
        self.sp = self.sp.wrapping_sub(2);
        bus.write_u16(self.sp, value).await;
    }

    pub(super) async fn run_pop(&mut self, bus: &mut Bus, dest: WordDest) {
        let value = bus.read_u16(self.sp).await;
        self.sp = self.sp.wrapping_add(2);
        self.write_word_dest(bus, dest, value).await;
    }

    pub(super) async fn run_inc8(&mut self, bus: &mut Bus, loc: ByteLocation) {
        let val = self.resolve_byte_source(bus, loc.into()).await;
        let new_val = val.wrapping_add(1);

        self.registers.f.zero = new_val == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (val & 0x0F) == 0x0F;

        self.write_byte_dest(bus, loc.into(), new_val).await;
    }

    pub(super) async fn run_dec8(&mut self, bus: &mut Bus, loc: ByteLocation) {
        let val = self.resolve_byte_source(bus, loc.into()).await;
        let new_val = val.wrapping_sub(1);

        self.registers.f.zero = new_val == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (val & 0x0F) == 0x00;

        self.write_byte_dest(bus, loc.into(), new_val).await;
    }

    pub(super) async fn run_inc16(&mut self, bus: &mut Bus, loc: WordDest) {
        let value = self.resolve_word_source(bus, self.word_dest_to_src(loc)).await;
        bus.tick().await; // 16-bit ALU internal cycle
        self.write_word_dest(bus, loc, value.wrapping_add(1)).await;
    }

    pub(super) async fn run_dec16(&mut self, bus: &mut Bus, loc: WordDest) {
        let value = self.resolve_word_source(bus, self.word_dest_to_src(loc)).await;
        bus.tick().await; // 16-bit ALU internal cycle
        self.write_word_dest(bus, loc, value.wrapping_sub(1)).await;
    }

    pub(super) async fn run_addhl(&mut self, bus: &mut Bus, source: WordSource) {
        let value = self.resolve_word_source(bus, source).await;
        bus.tick().await; // Internal cycle
        let hl = self.registers.get_hl();
        let (new_hl, carry) = hl.overflowing_add(value);

        self.registers.f.subtract = false;
        self.registers.f.half_carry = (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
        self.registers.f.carry = carry;

        self.registers.set_hl(new_hl);
    }

    pub(super) async fn run_addsp(&mut self, _bus: &mut Bus, val: i8) {
        let val_unsigned = val as u8;

        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (self.sp & 0x000F) + (val_unsigned & 0x0F) as u16 > 0x000F;
        self.registers.f.carry = (self.sp & 0x00FF) + val_unsigned as u16 > 0xFF;

        self.sp = self.sp.wrapping_add_signed(val as i16);
    }

    pub(super) async fn run_add(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let (new_a, carry) = a.overflowing_add(value);

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (a & 0x0F) + (value & 0x0F) > 0x0F;
        self.registers.f.carry = carry;

        self.registers.a = new_a;
    }

    pub(super) async fn run_adc(&mut self, bus: &mut Bus, source: ByteSource) {
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

    pub(super) async fn run_sub(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let (new_a, carry) = a.overflowing_sub(value);

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (a & 0x0F) < (value & 0x0F);
        self.registers.f.carry = carry;

        self.registers.a = new_a;
    }

    pub(super) async fn run_sbc(&mut self, bus: &mut Bus, source: ByteSource) {
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

    pub(super) async fn run_and(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let new_a = a & value;

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = true;
        self.registers.f.carry = false;

        self.registers.a = new_a;
    }

    pub(super) async fn run_xor(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let new_a = a ^ value;

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;

        self.registers.a = new_a;
    }

    pub(super) async fn run_or(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let new_a = a | value;

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;

        self.registers.a = new_a;
    }

    pub(super) async fn run_cp(&mut self, bus: &mut Bus, source: ByteSource) {
        let value = self.resolve_byte_source(bus, source).await;
        let a = self.registers.a;
        let (new_a, carry) = a.overflowing_sub(value);

        self.registers.f.zero = new_a == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (a & 0x0F) < (value & 0x0F);
        self.registers.f.carry = carry;
    }

    pub(super) async fn run_daa(&mut self) {
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

    pub(super) async fn run_scf(&mut self) {
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = true;
    }

    pub(super) async fn run_cpl(&mut self) {
        let a = self.registers.a;
        let new_a = !a;

        self.registers.f.subtract = true;
        self.registers.f.half_carry = true;

        self.registers.a = new_a;
    }

    pub(super) async fn run_ccf(&mut self) {
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = !self.registers.f.carry;
    }

    // opcode decode
    pub(super) fn decode_dest(&self, bits: u8) -> ByteDest {
        self.decode_bits_to_location(bits).into()
    }

    pub(super) fn decode_source(&self, bits: u8) -> ByteSource {
        self.decode_bits_to_location(bits).into()
    }

    pub(super) fn decode_bits_to_location(&self, bits: u8) -> ByteLocation {
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
}
