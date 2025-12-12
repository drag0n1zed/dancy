// The F register (or flags register) is 8 bits, but only the top 4 are used.

// Bit  Name  Set If...
// 7    Z     Result of operation is Zero.
// 6    N     The last operation was a Subtraction.
// 5    H     There was a carry from bit 3 to bit 4.
// 4    C     There was a carry from bit 7.
// 3-0        Not used (always read as 0).

#[derive(Default)]
pub struct FlagsRegister {
    pub zero: bool,
    pub subtract: bool,
    pub half_carry: bool,
    pub carry: bool,
}

impl FlagsRegister {
    pub fn to_u8(&self) -> u8 {
        (if self.zero {1} else {0} ) << 7
            | (if self.subtract {1} else {0}) << 6
            | (if self.half_carry {1} else {0}) << 5
            | (if self.carry {1} else {0}) << 4
    }
    
    pub fn from_u8(value: u8) -> Self {
        FlagsRegister {
            zero: (value >> 7) & 1 != 0,
            subtract: (value >> 6) & 1 != 0,
            half_carry: (value >> 5) & 1 != 0,
            carry: (value >> 4) & 1 != 0,
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
    pub fn new() -> Self {Default::default()}
    
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8)  | (self.f.to_u8() as u16)
    }
    
    pub fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        self.f = FlagsRegister::from_u8((value & 0x00F0) as u8);
    }
    
    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8)  | (self.c as u16)
    }
    
    pub fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0x00FF) as u8;
    }
    
    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8)  | (self.e as u16)
    }
    
    pub fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0x00FF) as u8;
    }
    
    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8)  | (self.l as u16)
    }
    
    pub fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0x00FF) as u8;
    }
}