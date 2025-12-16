use crate::mmu::Bus;

// 8-bit value source
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
    SPPlusImmediate(i8),
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
    POP(WordDest),
    PUSH(WordSource),

    // alu
    INC8(ByteDest),
    DEC8(ByteDest),
    INC16(WordDest),
    DEC16(WordDest),
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
    pub fn from_opcode(opcode: u8, bus: &Bus, pc: u16) -> (Self, u16) {
        let mut op_bytes: u16 = 1;
        let instruction = match opcode {
            0x00 => Instruction::NOP,
            0x10 => {
                get_byte_adv(bus, pc, &mut op_bytes); // Weird opcode, skips the next byte
                Instruction::STOP
            }
            0x20 => Instruction::JR(JumpCondition::NotZero, get_byte_adv(bus, pc, &mut op_bytes) as i8),
            0x30 => Instruction::JR(JumpCondition::NoCarry, get_byte_adv(bus, pc, &mut op_bytes) as i8),

            0x01 => Instruction::LD16(
                WordDest::BC,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            0x11 => Instruction::LD16(
                WordDest::DE,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            0x21 => Instruction::LD16(
                WordDest::HL,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            0x31 => Instruction::LD16(
                WordDest::SP,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),

            0x02 => Instruction::LD8(ByteDest::IndBC, ByteSource::A),
            0x12 => Instruction::LD8(ByteDest::IndDE, ByteSource::A),
            0x22 => Instruction::LD8(ByteDest::IndHLI, ByteSource::A),
            0x32 => Instruction::LD8(ByteDest::IndHLD, ByteSource::A),

            0x03 => Instruction::INC16(WordDest::BC),
            0x13 => Instruction::INC16(WordDest::DE),
            0x23 => Instruction::INC16(WordDest::HL),
            0x33 => Instruction::INC16(WordDest::SP),

            0x04 => Instruction::INC8(ByteDest::B),
            0x14 => Instruction::INC8(ByteDest::D),
            0x24 => Instruction::INC8(ByteDest::H),
            0x34 => Instruction::INC8(ByteDest::IndHL),

            0x05 => Instruction::DEC8(ByteDest::B),
            0x15 => Instruction::DEC8(ByteDest::D),
            0x25 => Instruction::DEC8(ByteDest::H),
            0x35 => Instruction::DEC8(ByteDest::IndHL),

            0x06 => Instruction::LD8(ByteDest::B, ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0x16 => Instruction::LD8(ByteDest::D, ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0x26 => Instruction::LD8(ByteDest::H, ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0x36 => Instruction::LD8(
                ByteDest::IndHL,
                ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes)),
            ),

            0x07 => Instruction::RLCA,
            0x17 => Instruction::RLA,
            0x27 => Instruction::DAA,
            0x37 => Instruction::SCF,

            0x08 => Instruction::LD16(WordDest::Address(get_word_adv(bus, pc, &mut op_bytes)), WordSource::SP),
            0x18 => Instruction::JR(JumpCondition::Always, get_byte_adv(bus, pc, &mut op_bytes) as i8),
            0x28 => Instruction::JR(JumpCondition::Zero, get_byte_adv(bus, pc, &mut op_bytes) as i8),
            0x38 => Instruction::JR(JumpCondition::Carry, get_byte_adv(bus, pc, &mut op_bytes) as i8),

            0x09 => Instruction::ADDHL(WordSource::BC),
            0x19 => Instruction::ADDHL(WordSource::DE),
            0x29 => Instruction::ADDHL(WordSource::HL),
            0x39 => Instruction::ADDHL(WordSource::SP),

            0x0A => Instruction::LD8(ByteDest::A, ByteSource::IndBC),
            0x1A => Instruction::LD8(ByteDest::A, ByteSource::IndDE),
            0x2A => Instruction::LD8(ByteDest::A, ByteSource::IndHLI),
            0x3A => Instruction::LD8(ByteDest::A, ByteSource::IndHLD),

            0x0B => Instruction::DEC16(WordDest::BC),
            0x1B => Instruction::DEC16(WordDest::DE),
            0x2B => Instruction::DEC16(WordDest::HL),
            0x3B => Instruction::DEC16(WordDest::SP),

            0x0C => Instruction::INC8(ByteDest::C),
            0x1C => Instruction::INC8(ByteDest::E),
            0x2C => Instruction::INC8(ByteDest::L),
            0x3C => Instruction::INC8(ByteDest::A),

            0x0D => Instruction::DEC8(ByteDest::C),
            0x1D => Instruction::DEC8(ByteDest::E),
            0x2D => Instruction::DEC8(ByteDest::L),
            0x3D => Instruction::DEC8(ByteDest::A),

            0x0E => Instruction::LD8(ByteDest::C, ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0x1E => Instruction::LD8(ByteDest::E, ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0x2E => Instruction::LD8(ByteDest::L, ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0x3E => Instruction::LD8(ByteDest::A, ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),

            0x0F => Instruction::RRCA,
            0x1F => Instruction::RRA,
            0x2F => Instruction::CPL,
            0x3F => Instruction::CCF,

            0x76 => Instruction::HALT,
            0x40..=0x7F => {
                // 0b01DDDSSS
                let source: ByteSource = decode_bits_to_location(opcode & 0b0000_0111).into();
                let destination: ByteDest = decode_bits_to_location((opcode & 0b0011_1000) >> 3).into();
                Instruction::LD8(destination, source)
            }

            0x80..=0xBF => {
                // 0b10IIISSS, where I is instruction
                let source: ByteSource = decode_bits_to_location(opcode & 0b0000_0111).into();
                match (opcode & 0b0011_1000) >> 3 {
                    0b000 => Instruction::ADD(source),
                    0b001 => Instruction::ADC(source),
                    0b010 => Instruction::SUB(source),
                    0b011 => Instruction::SBC(source),
                    0b100 => Instruction::AND(source),
                    0b101 => Instruction::XOR(source),
                    0b110 => Instruction::OR(source),
                    0b111 => Instruction::CP(source),
                    _ => unreachable!(),
                }
            }

            0xC0 => Instruction::RET(JumpCondition::NotZero),
            0xD0 => Instruction::RET(JumpCondition::NoCarry),
            0xE0 => Instruction::LD8(
                ByteDest::Address(0xFF00 + (get_byte_adv(bus, pc, &mut op_bytes)) as u16),
                ByteSource::A,
            ),
            0xF0 => Instruction::LD8(
                ByteDest::A,
                ByteSource::Address(0xFF00 + (get_byte_adv(bus, pc, &mut op_bytes)) as u16),
            ),

            0xC1 => Instruction::POP(WordDest::BC),
            0xD1 => Instruction::POP(WordDest::DE),
            0xE1 => Instruction::POP(WordDest::HL),
            0xF1 => Instruction::POP(WordDest::AF),

            0xC2 => Instruction::JP(
                JumpCondition::NotZero,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            0xD2 => Instruction::JP(
                JumpCondition::NoCarry,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            0xE2 => Instruction::LD8(ByteDest::FF00PlusC, ByteSource::A),
            0xF2 => Instruction::LD8(ByteDest::A, ByteSource::FF00PlusC),

            0xC3 => Instruction::JP(
                JumpCondition::Always,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            // 0xD3 => Illegal
            // 0xE3 => Illegal
            0xF3 => Instruction::DI,

            0xC4 => Instruction::CALL(JumpCondition::NotZero, get_word_adv(bus, pc, &mut op_bytes)),
            0xD4 => Instruction::CALL(JumpCondition::NoCarry, get_word_adv(bus, pc, &mut op_bytes)),
            // 0xE4 => Illegal
            // 0xF4 => Illegal
            //
            0xC5 => Instruction::PUSH(WordSource::BC),
            0xD5 => Instruction::PUSH(WordSource::DE),
            0xE5 => Instruction::PUSH(WordSource::HL),
            0xF5 => Instruction::PUSH(WordSource::AF),

            0xC6 => Instruction::ADD(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0xD6 => Instruction::SUB(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0xE6 => Instruction::AND(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0xF6 => Instruction::OR(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),

            0xC7 | 0xD7 | 0xE7 | 0xF7 => Instruction::RST(opcode & 0b0011_1000),

            0xC8 => Instruction::RET(JumpCondition::Zero),
            0xD8 => Instruction::RET(JumpCondition::Carry),
            0xE8 => Instruction::ADDSP((get_byte_adv(bus, pc, &mut op_bytes)) as i8),
            0xF8 => Instruction::LD16(
                WordDest::HL,
                WordSource::SPPlusImmediate(get_byte_adv(bus, pc, &mut op_bytes) as i8),
            ),

            0xC9 => Instruction::RET(JumpCondition::Always),
            0xD9 => Instruction::RETI,
            0xE9 => Instruction::JP(JumpCondition::Always, WordSource::HL),
            0xF9 => Instruction::LD16(WordDest::SP, WordSource::HL),

            0xCA => Instruction::JP(
                JumpCondition::Zero,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            0xDA => Instruction::JP(
                JumpCondition::Carry,
                WordSource::Immediate(get_word_adv(bus, pc, &mut op_bytes)),
            ),
            0xEA => Instruction::LD8(ByteDest::Address(get_word_adv(bus, pc, &mut op_bytes)), ByteSource::A),
            0xFA => Instruction::LD8(ByteDest::A, ByteSource::Address(get_word_adv(bus, pc, &mut op_bytes))),

            0xCB => {
                let cb_opcode = get_byte_adv(bus, pc, &mut op_bytes);
                let destination: ByteDest = decode_bits_to_location(cb_opcode & 0b0000_0111).into();
                let bit = (cb_opcode & 0b0011_1000) >> 3;
                match cb_opcode {
                    0x00..=0x07 => Instruction::RLC(destination),
                    0x08..=0x0F => Instruction::RRC(destination),
                    0x10..=0x17 => Instruction::RL(destination),
                    0x18..=0x1F => Instruction::RR(destination),
                    0x20..=0x27 => Instruction::SLA(destination),
                    0x28..=0x2F => Instruction::SRA(destination),
                    0x30..=0x37 => Instruction::SWAP(destination),
                    0x38..=0x3F => Instruction::SRL(destination),
                    0x40..=0x7F => Instruction::BIT(bit, destination),
                    0x80..=0xBF => Instruction::RES(bit, destination),
                    0xC0..=0xFF => Instruction::SET(bit, destination),
                }
            }
            // 0xDB => Illegal
            // 0xEB => Illegal
            0xFB => Instruction::EI,

            0xCC => Instruction::CALL(JumpCondition::Zero, get_word_adv(bus, pc, &mut op_bytes)),
            0xDC => Instruction::CALL(JumpCondition::Carry, get_word_adv(bus, pc, &mut op_bytes)),
            // 0xEC => Illegal
            // 0xFC => Illegal
            //
            0xCD => Instruction::CALL(JumpCondition::Always, get_word_adv(bus, pc, &mut op_bytes)),
            // 0xDD => Illegal
            // 0xED => Illegal
            // 0xFD => Illegal
            //
            0xCE => Instruction::ADC(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0xDE => Instruction::SBC(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0xEE => Instruction::XOR(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),
            0xFE => Instruction::CP(ByteSource::Immediate(get_byte_adv(bus, pc, &mut op_bytes))),

            0xCF | 0xDF | 0xEF | 0xFF => Instruction::RST(opcode & 0b0011_1000),

            0xD3 | 0xE3 | 0xE4 | 0xF4 | 0xDB | 0xEB | 0xEC | 0xFC | 0xDD | 0xED | 0xFD => {
                panic!("Illegal opcode encountered: {:#04X} at PC: {:#06X}", opcode, pc);
            }
        };
        (instruction, op_bytes)
    }
}

fn get_byte_adv(bus: &Bus, pc: u16, op_bytes: &mut u16) -> u8 {
    *op_bytes += 1;
    bus.read(pc.wrapping_add(1))
}

fn get_word_adv(bus: &Bus, pc: u16, op_bytes: &mut u16) -> u16 {
    *op_bytes += 2;
    let low = bus.read(pc.wrapping_add(1));

    let high = bus.read(pc.wrapping_add(2));

    // Little endian -> low first then high. when combined its high then low.
    u16::from_le_bytes([low, high])
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
