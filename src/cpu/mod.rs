mod executor;
mod opcodes;
mod registers;

use crate::cpu::opcodes::{ByteDest, ByteSource, JumpCondition, WordDest, WordLocation, WordSource};
use crate::cpu::registers::Registers;
use crate::mmu::Bus;

pub struct Cpu {
    pub registers: Registers,
    pub pc: u16,
    pub sp: u16,
    pub ime: bool,
    pub ime_countdown: u8,
    pub halted: bool,
    pub halt_bug_active: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            // Skip BOOT ROM
            registers: Registers::new(),
            pc: 0x0100,
            sp: 0xFFFE,
            ime: false,
            ime_countdown: 0,
            halted: false,
            halt_bug_active: false,
        }
    }

    async fn handle_interrupt(&mut self, bus: &mut Bus, pending: u8) {
        bus.tick().await;
        bus.tick().await;
        self.ime = false;
        self.sp = self.sp.wrapping_sub(2);
        bus.write_u16(self.sp, self.pc, false).await; // +2 cycles
        bus.tick().await;
        for bit in 0..5 {
            let target = 0b1 << bit;
            if pending & target != 0 {
                bus.interrupt_flag &= !target;
                self.pc = u16::from_le_bytes([0x40 + 0x08 * bit, 0x00]);
                return;
            }
        }
    }

    pub async fn step(&mut self, bus: &mut Bus) {
        // For EI, set IME to true AFTER the NEXT cycle.
        if self.ime_countdown > 0 {
            self.ime_countdown -= 1;
            if self.ime_countdown == 0 {
                self.ime = true;
            }
        }

        // If IME is set, CPU wakes up
        if !self.halted {
            let pending = bus.interrupt_enable & bus.interrupt_flag & 0x1F;
            if pending != 0 {
                if self.ime {
                    self.handle_interrupt(bus, pending).await;
                }
            }
        } else {
            // If halted
            bus.tick().await; // +1 cycle
            let pending = bus.interrupt_enable & bus.interrupt_flag & 0x1F; // Recalculate after the await
            if pending != 0 {
                self.halted = false;
                if self.ime {
                    self.handle_interrupt(bus, pending).await;
                }
                // No interrupt handling / flag reset if ime set to false.
            } else {
                return; // Wait until interrupt happens
            }
        }

        let opcode = bus.read(self.pc).await;
        self.pc = self.pc.wrapping_add(1);
        if self.halt_bug_active {
            // The opcode is read twice if HALT bug is active. Two scenarios:
            // 1. Single byte instruction. The instruction runs twice.
            // 2. Multi byte instruction. The opcode is read again as an operand.
            // e.g. `ld a, $14` ($3E $14) is read as `ld a, $3E` ($3E $3E) then `inc d` ($14).
            self.pc = self.pc.wrapping_sub(1);
            self.halt_bug_active = false;
        }

        match opcode {
            // NOP
            0x00 => {}

            // STOP
            0x10 => {
                // Minimal implementation. Commercial DMG games don't use STOP because it's very buggy.
                // https://gbdev.io/pandocs/Reducing_Power_Consumption.html#the-bizarre-case-of-the-game-boy-stop-instruction-before-even-considering-timing
                let pending = bus.interrupt_enable & bus.interrupt_flag & 0x1F;
                if pending == 0 {
                    let _discarded_byte = self.fetch_byte(bus).await;
                }
                log::warn!("STOP executed at pc: 0x{:X}", self.pc);
            }

            // HALT
            0x76 => {
                let pending = bus.interrupt_enable & bus.interrupt_flag & 0x1F;
                if !self.ime && pending != 0 {
                    // Does not enter HALT mode
                    self.halt_bug_active = true;
                } else {
                    self.halted = true;
                }
            }

            // DI / EI
            0xF3 => {
                self.ime = false;
                self.ime_countdown = 0; // Disable IME and cancel preceding EI instruction
            }
            0xFB => {
                self.ime_countdown = 2; // Enable IME AFTER the NEXT cycle
            }

            // LD r, r'
            0x40..=0x7F => {
                let dest: ByteDest = self.decode_bits_to_location((opcode >> 3) & 0b0000_0111).into();
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_ld8(bus, dest, src).await;
            }
            // LD r, n
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E => {
                let dest: ByteDest = self.decode_bits_to_location((opcode >> 3) & 0b0000_0111).into();
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
                let loc = self.decode_bits_to_location((opcode >> 3) & 0b0000_0111);
                self.run_inc8(bus, loc).await;
            }
            // DEC r
            0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => {
                let loc = self.decode_bits_to_location((opcode >> 3) & 0b0000_0111);
                self.run_dec8(bus, loc).await;
            }
            // ADD A, r
            0x80..=0x87 => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_add(bus, src).await;
            }
            // ADC A, r
            0x88..=0x8F => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_adc(bus, src).await;
            }
            // SUB A, r
            0x90..=0x97 => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_sub(bus, src).await;
            }
            // SBC A, r
            0x98..=0x9F => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_sbc(bus, src).await;
            }
            // AND A, r
            0xA0..=0xA7 => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_and(bus, src).await;
            }
            // XOR A, r
            0xA8..=0xAF => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_xor(bus, src).await;
            }
            // OR A, r
            0xB0..=0xB7 => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
                self.run_or(bus, src).await;
            }
            // CP A, r
            0xB8..=0xBF => {
                let src: ByteSource = self.decode_bits_to_location(opcode & 0b0000_0111).into();
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
            0x03 => self.run_inc16(bus, WordLocation::BC).await,
            0x13 => self.run_inc16(bus, WordLocation::DE).await,
            0x23 => self.run_inc16(bus, WordLocation::HL).await,
            0x33 => self.run_inc16(bus, WordLocation::SP).await,
            // DEC16
            0x0B => self.run_dec16(bus, WordLocation::BC).await,
            0x1B => self.run_dec16(bus, WordLocation::DE).await,
            0x2B => self.run_dec16(bus, WordLocation::HL).await,
            0x3B => self.run_dec16(bus, WordLocation::SP).await,
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
                let loc = self.decode_bits_to_location(cb_opcode & 0b0000_0111);
                let bit = (cb_opcode & 0b0011_1000) >> 3;
                match cb_opcode {
                    0x00..=0x07 => self.run_rlc(bus, loc).await,
                    0x08..=0x0F => self.run_rrc(bus, loc).await,
                    0x10..=0x17 => self.run_rl(bus, loc).await,
                    0x18..=0x1F => self.run_rr(bus, loc).await,
                    0x20..=0x27 => self.run_sla(bus, loc).await,
                    0x28..=0x2F => self.run_sra(bus, loc).await,
                    0x30..=0x37 => self.run_swap(bus, loc).await,
                    0x38..=0x3F => self.run_srl(bus, loc).await,
                    0x40..=0x7F => self.run_bit(bus, loc, bit).await,
                    0x80..=0xBF => self.run_res(bus, loc, bit).await,
                    0xC0..=0xFF => self.run_set(bus, loc, bit).await,
                }
            }

            // RLCA
            0x07 => self.run_rlca(bus).await,
            // RLA
            0x17 => self.run_rla(bus).await,
            // RRCA
            0x0F => self.run_rrca(bus).await,
            // RRA
            0x1F => self.run_rra(bus).await,

            0xD3 | 0xE3 | 0xE4 | 0xF4 | 0xDB | 0xEB | 0xEC | 0xFC | 0xDD | 0xED | 0xFD => {
                panic!("Illegal opcode: {:#04X}", opcode);
            }
        }
    }
}
