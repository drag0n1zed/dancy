pub struct Cartridge {
    rom_data: Vec<u8>,
    external_ram: [u8; 0x2000],
}

impl Cartridge {
    pub fn new(rom_data: Vec<u8>) -> Self {
        Self {
            rom_data,
            external_ram: [0; 0x2000],
        }
    }
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.rom_data[addr as usize],
            0xA000..=0xBFFF => self.external_ram[(addr - 0xA000) as usize],
            _ => unreachable!(),
        }
    }
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.rom_data[addr as usize] = value,
            0xA000..=0xBFFF => self.external_ram[(addr - 0xA000) as usize] = value,
            _ => unreachable!(),
        }
    }
}
