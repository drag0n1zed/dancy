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
#[derive(Clone, Copy)]
pub enum WordSource {
    AF,
    BC,
    DE,
    HL,
    SP,
    Immediate(u16),
}

// 16-bit value destination
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
pub enum JumpCondition {
    NotZero,
    Zero,
    NoCarry,
    Carry,
    Always,
}
