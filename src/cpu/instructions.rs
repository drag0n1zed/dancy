use crate::mmu::Bus;

// 8-bit value source
#[derive(PartialEq, Clone, Copy)]
pub enum ByteSource {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    IndBC,  // (BC)
    IndDE,  // (DE)
    IndHL,  // (HL)
    IndHLI, // (HL+)
    IndHLD, // (HL-)
    FF00PlusC,
    Address(u16),
    Immediate(u8),
}

// 8-bit value destination
#[derive(PartialEq, Clone, Copy)]
pub enum ByteDest {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    IndBC,
    IndDE,
    IndHL,
    IndHLI,
    IndHLD,
    FF00PlusC,
    Address(u16),
}

// 16-bit value source
pub enum WordSource {
    AF,
    BC,
    DE,
    HL,
    SP,
    Immediate(u16),
}

// 16-bit value destination
pub enum WordDest {
    AF,
    BC,
    DE,
    HL,
    SP,
    Address(u16),
}

// A helper enum, gets .into()d into Source / Destination enums
#[derive(Clone, Copy)]
pub enum ByteLocation {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    IndHL,
}

impl From<ByteLocation> for ByteSource {
    fn from(loc: ByteLocation) -> Self {
        match loc {
            ByteLocation::A => ByteSource::A,
            ByteLocation::B => ByteSource::B,
            ByteLocation::C => ByteSource::C,
            ByteLocation::D => ByteSource::D,
            ByteLocation::E => ByteSource::E,
            ByteLocation::H => ByteSource::H,
            ByteLocation::L => ByteSource::L,
            ByteLocation::IndHL => ByteSource::IndHL,
        }
    }
}

impl From<ByteLocation> for ByteDest {
    fn from(loc: ByteLocation) -> Self {
        match loc {
            ByteLocation::A => ByteDest::A,
            ByteLocation::B => ByteDest::B,
            ByteLocation::C => ByteDest::C,
            ByteLocation::D => ByteDest::D,
            ByteLocation::E => ByteDest::E,
            ByteLocation::H => ByteDest::H,
            ByteLocation::L => ByteDest::L,
            ByteLocation::IndHL => ByteDest::IndHL,
        }
    }
}
#[derive(Clone, Copy)]
pub enum WordLocation {
    BC,
    DE,
    HL,
    SP,
}

impl From<WordLocation> for WordSource {
    fn from(loc: WordLocation) -> Self {
        match loc {
            WordLocation::BC => WordSource::BC,
            WordLocation::DE => WordSource::DE,
            WordLocation::HL => WordSource::HL,
            WordLocation::SP => WordSource::SP,
        }
    }
}

impl From<WordLocation> for WordDest {
    fn from(loc: WordLocation) -> Self {
        match loc {
            WordLocation::BC => WordDest::BC,
            WordLocation::DE => WordDest::DE,
            WordLocation::HL => WordDest::HL,
            WordLocation::SP => WordDest::SP,
        }
    }
}

pub enum JumpCondition {
    NotZero,
    Zero,
    NoCarry,
    Carry,
    Always,
}

pub enum Instruction {
    // control/misc
    NOP,
    STOP,
    HALT,
    DI,
    EI,

    // control/br
    JR(JumpCondition, i8),
    JP(JumpCondition, WordSource),
    RET(JumpCondition),
    RETI,
    CALL(JumpCondition, u16),
    RST(u8),

    // lsm
    LD8(ByteDest, ByteSource),
    LD16(WordDest, WordSource),
    LDHL(i8),
    PUSH(WordSource),
    POP(WordDest),

    // alu
    INC8(ByteLocation),
    DEC8(ByteLocation),
    INC16(WordLocation),
    DEC16(WordLocation),
    ADDHL(WordSource),
    ADDSP(i8),
    // Register A as destination ->
    ADD(ByteSource),
    ADC(ByteSource),
    SUB(ByteSource),
    SBC(ByteSource),
    AND(ByteSource),
    XOR(ByteSource),
    OR(ByteSource),
    CP(ByteSource),
    // <- Register A as destination
    DAA,
    SCF,
    CPL,
    CCF,

    // rsb
    RLCA,
    RLA,
    RRCA,
    RRA,
    RLC(ByteDest),
    RRC(ByteDest),
    RL(ByteDest),
    RR(ByteDest),
    SLA(ByteDest),
    SRA(ByteDest),
    SWAP(ByteDest),
    SRL(ByteDest),
    BIT(u8, ByteDest),
    RES(u8, ByteDest),
    SET(u8, ByteDest),
}

impl Instruction {
    pub fn from_opcode(opcode: u8, bus: &Bus, pc: u16) -> (Self, u16, (u32, u32)) {
        let mut op_bytes: u16 = 1;
        let (instruction, cycles) = match opcode {
            0x00 => (Instruction::NOP, (4, 4)),
            0x10 => {
                get_byte(bus, pc, &mut op_bytes); // Weird opcode, skips the next byte
                (Instruction::STOP, (4, 4))
            }
            0x20 => (
                Instruction::JR(JumpCondition::NotZero, get_byte(bus, pc, &mut op_bytes) as i8),
                (8, 12),
            ),
            0x30 => (
                Instruction::JR(JumpCondition::NoCarry, get_byte(bus, pc, &mut op_bytes) as i8),
                (8, 12),
            ),

            0x01 => (
                Instruction::LD16(WordDest::BC, WordSource::Immediate(get_word(bus, pc, &mut op_bytes))),
                (12, 12),
            ),
            0x11 => (
                Instruction::LD16(WordDest::DE, WordSource::Immediate(get_word(bus, pc, &mut op_bytes))),
                (12, 12),
            ),
            0x21 => (
                Instruction::LD16(WordDest::HL, WordSource::Immediate(get_word(bus, pc, &mut op_bytes))),
                (12, 12),
            ),
            0x31 => (
                Instruction::LD16(WordDest::SP, WordSource::Immediate(get_word(bus, pc, &mut op_bytes))),
                (12, 12),
            ),

            0x02 => (Instruction::LD8(ByteDest::IndBC, ByteSource::A), (8, 8)),
            0x12 => (Instruction::LD8(ByteDest::IndDE, ByteSource::A), (8, 8)),
            0x22 => (Instruction::LD8(ByteDest::IndHLI, ByteSource::A), (8, 8)),
            0x32 => (Instruction::LD8(ByteDest::IndHLD, ByteSource::A), (8, 8)),

            0x03 => (Instruction::INC16(WordLocation::BC), (8, 8)),
            0x13 => (Instruction::INC16(WordLocation::DE), (8, 8)),
            0x23 => (Instruction::INC16(WordLocation::HL), (8, 8)),
            0x33 => (Instruction::INC16(WordLocation::SP), (8, 8)),

            0x04 => (Instruction::INC8(ByteLocation::B), (4, 4)),
            0x14 => (Instruction::INC8(ByteLocation::D), (4, 4)),
            0x24 => (Instruction::INC8(ByteLocation::H), (4, 4)),
            0x34 => (Instruction::INC8(ByteLocation::IndHL), (12, 12)),

            0x05 => (Instruction::DEC8(ByteLocation::B), (4, 4)),
            0x15 => (Instruction::DEC8(ByteLocation::D), (4, 4)),
            0x25 => (Instruction::DEC8(ByteLocation::H), (4, 4)),
            0x35 => (Instruction::DEC8(ByteLocation::IndHL), (12, 12)),

            0x06 => (
                Instruction::LD8(ByteDest::B, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0x16 => (
                Instruction::LD8(ByteDest::D, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0x26 => (
                Instruction::LD8(ByteDest::H, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0x36 => (
                Instruction::LD8(ByteDest::IndHL, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (12, 12),
            ),

            0x07 => (Instruction::RLCA, (4, 4)),
            0x17 => (Instruction::RLA, (4, 4)),
            0x27 => (Instruction::DAA, (4, 4)),
            0x37 => (Instruction::SCF, (4, 4)),

            0x08 => (
                Instruction::LD16(WordDest::Address(get_word(bus, pc, &mut op_bytes)), WordSource::SP),
                (20, 20),
            ),
            0x18 => (
                Instruction::JR(JumpCondition::Always, get_byte(bus, pc, &mut op_bytes) as i8),
                (12, 12),
            ),
            0x28 => (
                Instruction::JR(JumpCondition::Zero, get_byte(bus, pc, &mut op_bytes) as i8),
                (8, 12),
            ),
            0x38 => (
                Instruction::JR(JumpCondition::Carry, get_byte(bus, pc, &mut op_bytes) as i8),
                (8, 12),
            ),

            0x09 => (Instruction::ADDHL(WordSource::BC), (8, 8)),
            0x19 => (Instruction::ADDHL(WordSource::DE), (8, 8)),
            0x29 => (Instruction::ADDHL(WordSource::HL), (8, 8)),
            0x39 => (Instruction::ADDHL(WordSource::SP), (8, 8)),

            0x0A => (Instruction::LD8(ByteDest::A, ByteSource::IndBC), (8, 8)),
            0x1A => (Instruction::LD8(ByteDest::A, ByteSource::IndDE), (8, 8)),
            0x2A => (Instruction::LD8(ByteDest::A, ByteSource::IndHLI), (8, 8)),
            0x3A => (Instruction::LD8(ByteDest::A, ByteSource::IndHLD), (8, 8)),

            0x0B => (Instruction::DEC16(WordLocation::BC), (8, 8)),
            0x1B => (Instruction::DEC16(WordLocation::DE), (8, 8)),
            0x2B => (Instruction::DEC16(WordLocation::HL), (8, 8)),
            0x3B => (Instruction::DEC16(WordLocation::SP), (8, 8)),

            0x0C => (Instruction::INC8(ByteLocation::C), (4, 4)),
            0x1C => (Instruction::INC8(ByteLocation::E), (4, 4)),
            0x2C => (Instruction::INC8(ByteLocation::L), (4, 4)),
            0x3C => (Instruction::INC8(ByteLocation::A), (4, 4)),

            0x0D => (Instruction::DEC8(ByteLocation::C), (4, 4)),
            0x1D => (Instruction::DEC8(ByteLocation::E), (4, 4)),
            0x2D => (Instruction::DEC8(ByteLocation::L), (4, 4)),
            0x3D => (Instruction::DEC8(ByteLocation::A), (4, 4)),

            0x0E => (
                Instruction::LD8(ByteDest::C, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0x1E => (
                Instruction::LD8(ByteDest::E, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0x2E => (
                Instruction::LD8(ByteDest::L, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0x3E => (
                Instruction::LD8(ByteDest::A, ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),

            0x0F => (Instruction::RRCA, (4, 4)),
            0x1F => (Instruction::RRA, (4, 4)),
            0x2F => (Instruction::CPL, (4, 4)),
            0x3F => (Instruction::CCF, (4, 4)),

            0x76 => (Instruction::HALT, (4, 4)),
            0x40..=0x7F => {
                // 0b01DDDSSS
                let source: ByteSource = decode_bits_to_location(opcode & 0b0000_0111).into();
                let destination: ByteDest = decode_bits_to_location((opcode & 0b0011_1000) >> 3).into();
                let cycle = if source == ByteSource::IndHL || destination == ByteDest::IndHL {
                    8
                } else {
                    4
                };
                (Instruction::LD8(destination, source), (cycle, cycle))
            }

            0x80..=0xBF => {
                // 0b10IIISSS, where I is Instruction
                let source: ByteSource = decode_bits_to_location(opcode & 0b0000_0111).into();
                let cycle = if source == ByteSource::IndHL { 8 } else { 4 };
                match (opcode & 0b0011_1000) >> 3 {
                    0b000 => (Instruction::ADD(source), (cycle, cycle)),
                    0b001 => (Instruction::ADC(source), (cycle, cycle)),
                    0b010 => (Instruction::SUB(source), (cycle, cycle)),
                    0b011 => (Instruction::SBC(source), (cycle, cycle)),
                    0b100 => (Instruction::AND(source), (cycle, cycle)),
                    0b101 => (Instruction::XOR(source), (cycle, cycle)),
                    0b110 => (Instruction::OR(source), (cycle, cycle)),
                    0b111 => (Instruction::CP(source), (cycle, cycle)),
                    _ => unreachable!(),
                }
            }

            0xC0 => (Instruction::RET(JumpCondition::NotZero), (8, 20)),
            0xD0 => (Instruction::RET(JumpCondition::NoCarry), (8, 20)),
            0xE0 => (
                Instruction::LD8(
                    ByteDest::Address(0xFF00 + get_byte(bus, pc, &mut op_bytes) as u16),
                    ByteSource::A,
                ),
                (12, 12),
            ),
            0xF0 => (
                Instruction::LD8(
                    ByteDest::A,
                    ByteSource::Address(0xFF00 + get_byte(bus, pc, &mut op_bytes) as u16),
                ),
                (12, 12),
            ),

            0xC1 => (Instruction::POP(WordDest::BC), (12, 12)),
            0xD1 => (Instruction::POP(WordDest::DE), (12, 12)),
            0xE1 => (Instruction::POP(WordDest::HL), (12, 12)),
            0xF1 => (Instruction::POP(WordDest::AF), (12, 12)),

            0xC2 => (
                Instruction::JP(
                    JumpCondition::NotZero,
                    WordSource::Immediate(get_word(bus, pc, &mut op_bytes)),
                ),
                (12, 16),
            ),
            0xD2 => (
                Instruction::JP(
                    JumpCondition::NoCarry,
                    WordSource::Immediate(get_word(bus, pc, &mut op_bytes)),
                ),
                (12, 16),
            ),
            0xE2 => (Instruction::LD8(ByteDest::FF00PlusC, ByteSource::A), (8, 8)),
            0xF2 => (Instruction::LD8(ByteDest::A, ByteSource::FF00PlusC), (8, 8)),

            0xC3 => (
                Instruction::JP(
                    JumpCondition::Always,
                    WordSource::Immediate(get_word(bus, pc, &mut op_bytes)),
                ),
                (16, 16),
            ),
            // 0xD3 => Illegal
            // 0xE3 => Illegal
            0xF3 => (Instruction::DI, (4, 4)),

            0xC4 => (
                Instruction::CALL(JumpCondition::NotZero, get_word(bus, pc, &mut op_bytes)),
                (12, 24),
            ),
            0xD4 => (
                Instruction::CALL(JumpCondition::NoCarry, get_word(bus, pc, &mut op_bytes)),
                (12, 24),
            ),
            // 0xE4 => Illegal
            // 0xF4 => Illegal
            //
            0xC5 => (Instruction::PUSH(WordSource::BC), (16, 16)),
            0xD5 => (Instruction::PUSH(WordSource::DE), (16, 16)),
            0xE5 => (Instruction::PUSH(WordSource::HL), (16, 16)),
            0xF5 => (Instruction::PUSH(WordSource::AF), (16, 16)),

            0xC6 => (
                Instruction::ADD(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0xD6 => (
                Instruction::SUB(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0xE6 => (
                Instruction::AND(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0xF6 => (
                Instruction::OR(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),

            0xC7 | 0xD7 | 0xE7 | 0xF7 => (Instruction::RST(opcode & 0b0011_1000), (16, 16)),

            0xC8 => (Instruction::RET(JumpCondition::Zero), (8, 20)),
            0xD8 => (Instruction::RET(JumpCondition::Carry), (8, 20)),
            0xE8 => (Instruction::ADDSP(get_byte(bus, pc, &mut op_bytes) as i8), (16, 16)),
            0xF8 => (Instruction::LDHL(get_byte(bus, pc, &mut op_bytes) as i8), (12, 12)),

            0xC9 => (Instruction::RET(JumpCondition::Always), (16, 16)),
            0xD9 => (Instruction::RETI, (16, 16)),
            0xE9 => (Instruction::JP(JumpCondition::Always, WordSource::HL), (4, 4)),
            0xF9 => (Instruction::LD16(WordDest::SP, WordSource::HL), (8, 8)),

            0xCA => (
                Instruction::JP(
                    JumpCondition::Zero,
                    WordSource::Immediate(get_word(bus, pc, &mut op_bytes)),
                ),
                (12, 16),
            ),
            0xDA => (
                Instruction::JP(
                    JumpCondition::Carry,
                    WordSource::Immediate(get_word(bus, pc, &mut op_bytes)),
                ),
                (12, 16),
            ),
            0xEA => (
                Instruction::LD8(ByteDest::Address(get_word(bus, pc, &mut op_bytes)), ByteSource::A),
                (16, 16),
            ),
            0xFA => (
                Instruction::LD8(ByteDest::A, ByteSource::Address(get_word(bus, pc, &mut op_bytes))),
                (16, 16),
            ),

            0xCB => {
                let cb_opcode = get_byte(bus, pc, &mut op_bytes);
                let destination: ByteDest = decode_bits_to_location(cb_opcode & 0b0000_0111).into();
                let cycle = if destination == ByteDest::IndHL {
                    if (0x40..=0x7F).contains(&cb_opcode) { 12 } else { 16 } // BIT (HL) uses 12 cycles
                } else {
                    8
                };
                let bit = (cb_opcode & 0b0011_1000) >> 3;
                match cb_opcode {
                    0x00..=0x07 => (Instruction::RLC(destination), (cycle, cycle)),
                    0x08..=0x0F => (Instruction::RRC(destination), (cycle, cycle)),
                    0x10..=0x17 => (Instruction::RL(destination), (cycle, cycle)),
                    0x18..=0x1F => (Instruction::RR(destination), (cycle, cycle)),
                    0x20..=0x27 => (Instruction::SLA(destination), (cycle, cycle)),
                    0x28..=0x2F => (Instruction::SRA(destination), (cycle, cycle)),
                    0x30..=0x37 => (Instruction::SWAP(destination), (cycle, cycle)),
                    0x38..=0x3F => (Instruction::SRL(destination), (cycle, cycle)),
                    0x40..=0x7F => (Instruction::BIT(bit, destination), (cycle, cycle)),
                    0x80..=0xBF => (Instruction::RES(bit, destination), (cycle, cycle)),
                    0xC0..=0xFF => (Instruction::SET(bit, destination), (cycle, cycle)),
                }
            }
            // 0xDB => Illegal
            // 0xEB => Illegal
            0xFB => (Instruction::EI, (4, 4)),

            0xCC => (
                Instruction::CALL(JumpCondition::Zero, get_word(bus, pc, &mut op_bytes)),
                (12, 24),
            ),
            0xDC => (
                Instruction::CALL(JumpCondition::Carry, get_word(bus, pc, &mut op_bytes)),
                (12, 24),
            ),
            // 0xEC => Illegal
            // 0xFC => Illegal
            //
            0xCD => (
                Instruction::CALL(JumpCondition::Always, get_word(bus, pc, &mut op_bytes)),
                (24, 24),
            ),
            // 0xDD => Illegal
            // 0xED => Illegal
            // 0xFD => Illegal
            //
            0xCE => (
                Instruction::ADC(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0xDE => (
                Instruction::SBC(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0xEE => (
                Instruction::XOR(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),
            0xFE => (
                Instruction::CP(ByteSource::Immediate(get_byte(bus, pc, &mut op_bytes))),
                (8, 8),
            ),

            0xCF | 0xDF | 0xEF | 0xFF => (Instruction::RST(opcode & 0b0011_1000), (16, 16)),

            0xD3 | 0xE3 | 0xE4 | 0xF4 | 0xDB | 0xEB | 0xEC | 0xFC | 0xDD | 0xED | 0xFD => {
                panic!("Illegal opcode encountered: {:#04X} at PC: {:#06X}", opcode, pc);
            }
        };
        (instruction, op_bytes, cycles)
    }
}

fn get_byte(bus: &Bus, pc: u16, op_bytes: &mut u16) -> u8 {
    *op_bytes += 1;
    bus.read(pc.wrapping_add(1))
}

fn get_word(bus: &Bus, pc: u16, op_bytes: &mut u16) -> u16 {
    *op_bytes += 2;
    bus.read_u16(pc.wrapping_add(1))
}

fn decode_bits_to_location(bits: u8) -> ByteLocation {
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
