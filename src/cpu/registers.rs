#[derive(Copy, Clone)]
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
        Self {
            a: 0x01,
            f: 0xB0.into(),
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
        }
    }

    pub fn get_af(&self) -> u16 {
        let flags: u8 = self.f.into();
        u16::from_le_bytes([flags, self.a])
    }

    pub fn set_af(&mut self, value: u16) {
        let [flags, a] = value.to_le_bytes();
        self.a = a;
        self.f = flags.into();
    }

    pub fn get_bc(&self) -> u16 {
        u16::from_le_bytes([self.c, self.b])
    }

    pub fn set_bc(&mut self, value: u16) {
        let [c, b] = value.to_le_bytes();
        self.b = b;
        self.c = c;
    }

    pub fn get_de(&self) -> u16 {
        u16::from_le_bytes([self.e, self.d])
    }

    pub fn set_de(&mut self, value: u16) {
        let [e, d] = value.to_le_bytes();
        self.d = d;
        self.e = e;
    }

    pub fn get_hl(&self) -> u16 {
        u16::from_le_bytes([self.l, self.h])
    }

    pub fn set_hl(&mut self, value: u16) {
        let [l, h] = value.to_le_bytes();
        self.h = h;
        self.l = l;
    }
}
