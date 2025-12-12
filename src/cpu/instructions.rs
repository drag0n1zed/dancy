use crate::mmu::Bus;

pub enum Target {
    A, B, C, D, E, G, H, L,
    BC, DE, HL, SP,
    HLI, HLD,
    Address(u16),
    Immediate8(u8),
    Immediate16(u16),
}

pub enum Instruction {
    NOP,
    LD(Target, Target),
    INC(Target),
    DEC(Target),
}

impl Instruction {

    pub fn from_opcode(opcode: u8, bus: &Bus, pc: &mut u16) -> Self {
        match opcode {
            0x00 => Instruction::NOP,
            0x01 => Instruction::LD(Target::BC, Target::Immediate16(next_two_bytes(bus, pc))),
            0x06 => Instruction::LD(Target::B, Target::Immediate8(next_byte(bus, pc))),
            _ => panic!("Unknown instruction 0x{:02X}", opcode),
        }
    }
}

fn next_byte(bus: &Bus, pc: &mut u16) -> u8 {
    let value = bus.read(*pc);
    *pc = pc.wrapping_add(1);
    value
}

fn next_two_bytes(bus: &Bus, pc: &mut u16) -> u16 {
    let low = bus.read(*pc);
    *pc = pc.wrapping_add(1);

    let high = bus.read(*pc);
    *pc = pc.wrapping_add(1);

    // Little endian -> swap order
    u16::from_le_bytes([low, high])
}

