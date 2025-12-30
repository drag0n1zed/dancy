use super::Mbc;

pub struct Mbc0 {
    rom: Vec<u8>,
}
impl Mbc0 {
    pub fn new(rom_data: Vec<u8>) -> Self {
        Mbc0 { rom: rom_data }
    }
}
impl Mbc for Mbc0 {
    fn read_rom(&self, addr: u16) -> u8 {
        self.rom[addr as usize]
    }
    fn write_rom(&mut self, _addr: u16, _value: u8) {
        // MBC0 has no registers to write to
    }
    fn read_ram(&self, _addr: u16) -> u8 {
        0xFF
    }
    fn write_ram(&mut self, _addr: u16, _value: u8) {
        // No support for MBC0 + RAM
    }
}
