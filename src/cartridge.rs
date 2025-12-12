pub struct Cartridge {
    rom_data: Vec<u8>,
}

impl Cartridge {
    pub fn new(rom_data: Vec<u8>) -> Self {
        Self { rom_data }
    }
    pub fn read(&self, addr: u16) -> u8 {
        self.rom_data[addr as usize]
    }
    pub fn write(&mut self, addr: u16, value: u8) {
        self.rom_data[addr as usize] = value;
    }
}
