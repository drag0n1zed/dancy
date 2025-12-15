#[derive(Default, Copy, Clone)]
pub struct FlagsRegister {
    pub zero: bool,
    pub subtract: bool,
    pub half_carry: bool,
    pub carry: bool,
}

// Only the top 4 bits are used.
// Bit  Name  Set If...
// 7    Z     Result of operation is Zero.
// 6    N     The last operation was a Subtraction.
// 5    H     There was a carry from bit 3 to bit 4.
// 4    C     There was a carry from bit 7.

impl From<FlagsRegister> for u8 {
    fn from(flags: FlagsRegister) -> u8 {
        (if flags.zero { 1 } else { 0 }) << 7
            | (if flags.subtract { 1 } else { 0 }) << 6
            | (if flags.half_carry { 1 } else { 0 }) << 5
            | (if flags.carry { 1 } else { 0 }) << 4
    }
}
impl From<u8> for FlagsRegister {
    fn from(byte: u8) -> FlagsRegister {
        FlagsRegister {
            zero: byte & 0b1000_0000 != 0,
            subtract: byte & 0b0100_0000 != 0,
            half_carry: byte & 0b0010_0000 != 0,
            carry: byte & 0b0001_0000 != 0,
        }
    }
}

#[derive(Default)]
pub struct Registers {
    pub a: u8,
    pub f: FlagsRegister,

    pub b: u8,
    pub c: u8,

    pub d: u8,
    pub e: u8,

    pub h: u8,
    pub l: u8,
}

impl Registers {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_af(&self) -> u16 {
        let flags: u8 = self.f.into();
        ((self.a as u16) << 8) | flags as u16
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        self.f = ((value & 0x00F0) as u8).into();
    }

    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0x00FF) as u8;
    }

    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0x00FF) as u8;
    }

    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0x00FF) as u8;
    }
}
