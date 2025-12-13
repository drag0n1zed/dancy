use crate::mmu::Bus;

pub enum Target {
    // Registers
    A,
    F,
    B,
    C,
    D,
    E,
    H,
    L,

    // Pairs
    AF,
    BC,
    IndBC, // (BC)
    DE,
    IndDE, // (DE)
    HL,
    IndHL,  // (HL)
    IndHLI, // (HL+)
    IndHLD, // (HL-)
    SP,

    // Data
    Address(u16),
    Immediate8(u8),
    SignedImmediate8(i8),
    Immediate16(u16),

    // The weird ones
    FF00PlusC,            // 0xE2
    SPPlusImmediate8(i8), // 0xF8
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
    JR(JumpCondition, Target),
    JP(JumpCondition, Target),
    RET(JumpCondition),
    RETI,
    CALL(JumpCondition, Target),
    RST(Target),

    // lsm
    LD(Target, Target),
    POP(Target),
    PUSH(Target),

    // alu
    INC(Target),
    DEC(Target),
    ADDHL(Target),
    ADDSP(Target),
    // Register A as destination ->
    ADD(Target),
    ADC(Target),
    SUB(Target),
    SBC(Target),
    AND(Target),
    XOR(Target),
    OR(Target),
    CP(Target),
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
    RLC(Target),
    RRC(Target),
    RL(Target),
    RR(Target),
    SLA(Target),
    SRA(Target),
    SWAP(Target),
    SRL(Target),
    BIT(u8, Target),
    RES(u8, Target),
    SET(u8, Target),
}

impl Instruction {
    pub fn from_opcode(opcode: u8, bus: &mut Bus, pc: &mut u16) -> Self {
        match opcode {
            0x00 => Instruction::NOP,
            0x10 => {
                next_byte(bus, pc); // Weird opcode, skips the next byte
                Instruction::STOP
            }
            0x20 => Instruction::JR(
                JumpCondition::NotZero,
                Target::SignedImmediate8(next_byte(bus, pc) as i8),
            ),
            0x30 => Instruction::JR(
                JumpCondition::NoCarry,
                Target::SignedImmediate8(next_byte(bus, pc) as i8),
            ),

            0x01 => Instruction::LD(Target::BC, Target::Immediate16(next_two_bytes(bus, pc))),
            0x11 => Instruction::LD(Target::DE, Target::Immediate16(next_two_bytes(bus, pc))),
            0x21 => Instruction::LD(Target::HL, Target::Immediate16(next_two_bytes(bus, pc))),
            0x31 => Instruction::LD(Target::SP, Target::Immediate16(next_two_bytes(bus, pc))),

            0x02 => Instruction::LD(Target::IndBC, Target::A),
            0x12 => Instruction::LD(Target::IndDE, Target::A),
            0x22 => Instruction::LD(Target::IndHLI, Target::A),
            0x32 => Instruction::LD(Target::IndHLD, Target::A),

            0x03 => Instruction::INC(Target::BC),
            0x13 => Instruction::INC(Target::DE),
            0x23 => Instruction::INC(Target::HL),
            0x33 => Instruction::INC(Target::SP),

            0x04 => Instruction::INC(Target::B),
            0x14 => Instruction::INC(Target::D),
            0x24 => Instruction::INC(Target::H),
            0x34 => Instruction::INC(Target::IndHL),

            0x05 => Instruction::DEC(Target::B),
            0x15 => Instruction::DEC(Target::D),
            0x25 => Instruction::DEC(Target::H),
            0x35 => Instruction::DEC(Target::IndHL),

            0x06 => Instruction::LD(Target::B, Target::Immediate8(next_byte(bus, pc))),
            0x16 => Instruction::LD(Target::D, Target::Immediate8(next_byte(bus, pc))),
            0x26 => Instruction::LD(Target::H, Target::Immediate8(next_byte(bus, pc))),
            0x36 => Instruction::LD(Target::IndHL, Target::Immediate8(next_byte(bus, pc))),

            0x07 => Instruction::RLCA,
            0x17 => Instruction::RLA,
            0x27 => Instruction::DAA,
            0x37 => Instruction::SCF,

            0x08 => Instruction::LD(Target::Address(next_two_bytes(bus, pc)), Target::SP),
            0x18 => Instruction::JR(
                JumpCondition::Always,
                Target::SignedImmediate8(next_byte(bus, pc) as i8),
            ),
            0x28 => Instruction::JR(JumpCondition::Zero, Target::SignedImmediate8(next_byte(bus, pc) as i8)),
            0x38 => Instruction::JR(JumpCondition::Carry, Target::SignedImmediate8(next_byte(bus, pc) as i8)),

            0x09 => Instruction::ADDHL(Target::BC),
            0x19 => Instruction::ADDHL(Target::DE),
            0x29 => Instruction::ADDHL(Target::HL),
            0x39 => Instruction::ADDHL(Target::SP),

            0x0A => Instruction::LD(Target::A, Target::IndBC),
            0x1A => Instruction::LD(Target::A, Target::IndDE),
            0x2A => Instruction::LD(Target::A, Target::IndHLI),
            0x3A => Instruction::LD(Target::A, Target::IndHLD),

            0x0B => Instruction::DEC(Target::BC),
            0x1B => Instruction::DEC(Target::DE),
            0x2B => Instruction::DEC(Target::HL),
            0x3B => Instruction::DEC(Target::SP),

            0x0C => Instruction::INC(Target::C),
            0x1C => Instruction::INC(Target::E),
            0x2C => Instruction::INC(Target::L),
            0x3C => Instruction::INC(Target::A),

            0x0D => Instruction::DEC(Target::C),
            0x1D => Instruction::DEC(Target::E),
            0x2D => Instruction::DEC(Target::L),
            0x3D => Instruction::DEC(Target::A),

            0x0E => Instruction::LD(Target::C, Target::Immediate8(next_byte(bus, pc))),
            0x1E => Instruction::LD(Target::E, Target::Immediate8(next_byte(bus, pc))),
            0x2E => Instruction::LD(Target::L, Target::Immediate8(next_byte(bus, pc))),
            0x3E => Instruction::LD(Target::A, Target::Immediate8(next_byte(bus, pc))),

            0x0F => Instruction::RRCA,
            0x1F => Instruction::RRA,
            0x2F => Instruction::CPL,
            0x3F => Instruction::CCF,

            0x76 => Instruction::HALT,
            0x40..=0x7F => {
                // 0b01DDDSSS
                let source = decode_bits_to_register(opcode & 0b0000_0111);
                let destination = decode_bits_to_register((opcode & 0b0011_1000) >> 3);
                Instruction::LD(destination, source)
            }

            0x80..=0xBF => {
                // 0b10IIISSS, where I is instruction
                let source = decode_bits_to_register(opcode & 0b0000_0111);
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
            0xE0 => Instruction::LD(Target::Address(0xFF00 + (next_byte(bus, pc)) as u16), Target::A),
            0xF0 => Instruction::LD(Target::A, Target::Address(0xFF00 + (next_byte(bus, pc)) as u16)),

            0xC1 => Instruction::POP(Target::BC),
            0xD1 => Instruction::POP(Target::DE),
            0xE1 => Instruction::POP(Target::HL),
            0xF1 => Instruction::POP(Target::AF),

            0xC2 => Instruction::JP(JumpCondition::NotZero, Target::Immediate16(next_two_bytes(bus, pc))),
            0xD2 => Instruction::JP(JumpCondition::NoCarry, Target::Immediate16(next_two_bytes(bus, pc))),
            0xE2 => Instruction::LD(Target::FF00PlusC, Target::A),
            0xF2 => Instruction::LD(Target::A, Target::FF00PlusC),

            0xC3 => Instruction::JP(JumpCondition::Always, Target::Immediate16(next_two_bytes(bus, pc))),
            // 0xD3 => Illegal
            // 0xE3 => Illegal
            0xF3 => Instruction::DI,

            0xC4 => Instruction::CALL(JumpCondition::NotZero, Target::Immediate16(next_two_bytes(bus, pc))),
            0xD4 => Instruction::CALL(JumpCondition::NoCarry, Target::Immediate16(next_two_bytes(bus, pc))),
            // 0xE4 => Illegal
            // 0xF4 => Illegal
            //
            0xC5 => Instruction::PUSH(Target::BC),
            0xD5 => Instruction::PUSH(Target::DE),
            0xE5 => Instruction::PUSH(Target::HL),
            0xF5 => Instruction::PUSH(Target::AF),

            0xC6 => Instruction::ADD(Target::Immediate8(next_byte(bus, pc))),
            0xD6 => Instruction::SUB(Target::Immediate8(next_byte(bus, pc))),
            0xE6 => Instruction::AND(Target::Immediate8(next_byte(bus, pc))),
            0xF6 => Instruction::OR(Target::Immediate8(next_byte(bus, pc))),

            0xC7 | 0xD7 | 0xE7 | 0xF7 => Instruction::RST(Target::Address((opcode & 0b0011_1000) as u16)),

            0xC8 => Instruction::RET(JumpCondition::Zero),
            0xD8 => Instruction::RET(JumpCondition::Carry),
            0xE8 => Instruction::ADDSP(Target::SignedImmediate8((next_byte(bus, pc)) as i8)),
            0xF8 => Instruction::LD(Target::HL, Target::SPPlusImmediate8(next_byte(bus, pc) as i8)),

            0xC9 => Instruction::RET(JumpCondition::Always),
            0xD9 => Instruction::RETI,
            0xE9 => Instruction::JP(JumpCondition::Always, Target::HL),
            0xF9 => Instruction::LD(Target::SP, Target::HL),

            0xCA => Instruction::JP(JumpCondition::Zero, Target::Immediate16(next_two_bytes(bus, pc))),
            0xDA => Instruction::JP(JumpCondition::Carry, Target::Immediate16(next_two_bytes(bus, pc))),
            0xEA => Instruction::LD(Target::Address(next_two_bytes(bus, pc)), Target::A),
            0xFA => Instruction::LD(Target::A, Target::Address(next_two_bytes(bus, pc))),

            0xCB => {
                let cb_opcode = next_byte(bus, pc);
                let destination = decode_bits_to_register(cb_opcode & 0b0000_0111);
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

            0xCC => Instruction::CALL(JumpCondition::Zero, Target::Immediate16(next_two_bytes(bus, pc))),
            0xDC => Instruction::CALL(JumpCondition::Carry, Target::Immediate16(next_two_bytes(bus, pc))),
            // 0xEC => Illegal
            // 0xFC => Illegal
            //
            0xCD => Instruction::CALL(JumpCondition::Always, Target::Immediate16(next_two_bytes(bus, pc))),
            // 0xDD => Illegal
            // 0xED => Illegal
            // 0xFD => Illegal
            //
            0xCE => Instruction::ADC(Target::Immediate8(next_byte(bus, pc))),
            0xDE => Instruction::SBC(Target::Immediate8(next_byte(bus, pc))),
            0xEE => Instruction::XOR(Target::Immediate8(next_byte(bus, pc))),
            0xFE => Instruction::CP(Target::Immediate8(next_byte(bus, pc))),

            0xCF | 0xDF | 0xEF | 0xFF => Instruction::RST(Target::Address((opcode & 0b0011_1000) as u16)),

            0xD3 | 0xE3 | 0xE4 | 0xF4 | 0xDB | 0xEB | 0xEC | 0xFC | 0xDD | 0xED | 0xFD => {
                panic!("Illegal opcode encountered: {:#04X} at PC: {:#06X}", opcode, *pc - 1);
            }
        }
    }
}

fn next_byte(bus: &mut Bus, pc: &mut u16) -> u8 {
    let value = bus.read(*pc);
    *pc = pc.wrapping_add(1);
    value
}

fn next_two_bytes(bus: &mut Bus, pc: &mut u16) -> u16 {
    let low = bus.read(*pc);
    *pc = pc.wrapping_add(1);

    let high = bus.read(*pc);
    *pc = pc.wrapping_add(1);

    // Little endian -> low first then high. when combined its high then low.
    u16::from_le_bytes([low, high])
}

fn decode_bits_to_register(bits: u8) -> Target {
    match bits {
        0b000 => Target::B,
        0b001 => Target::C,
        0b010 => Target::D,
        0b011 => Target::E,
        0b100 => Target::H,
        0b101 => Target::L,
        0b110 => Target::IndHL,
        0b111 => Target::A,
        _ => unreachable!(),
    }
}
