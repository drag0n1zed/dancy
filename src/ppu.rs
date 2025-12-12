pub struct Ppu {
    vram: [u8; 8192],
    oam: [u8; 160],
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: [0; 8192],
            oam: [0; 160],
        }
    }
    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[(addr - 0x8000) as usize]
    }
    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[(addr - 0x8000) as usize] = value
    }
    pub fn read_oam(&self, addr: u16) -> u8 {
        self.oam[(addr - 0xFE00) as usize]
    }
    pub fn write_oam(&mut self, addr: u16, value: u8) {
        self.oam[(addr - 0xFE00) as usize] = value;
    }
    pub fn read_register(&self, addr: u16) -> u8 {
        0
        // TODO
    }
    pub fn write_register(&mut self, addr: u16, value: u8) {
        // TODO
    }
}
