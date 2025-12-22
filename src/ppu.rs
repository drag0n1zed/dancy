pub struct Ppu {
    vram: [u8; 8192],
    oam: [u8; 160],
    back_buffer: [u8; 160 * 144 * 4],
    pub front_buffer: [u8; 160 * 144 * 4],
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: [0; 8192],
            oam: [0; 160],
            back_buffer: [0; 160 * 144 * 4],
            front_buffer: [0; 160 * 144 * 4],
        }
    }

    pub fn step(&mut self, t_cycles: u32) {
        todo!("Update LCD state and draw pixels to back_buffer");
    }

    pub fn update_front_buffer(&mut self) {
        std::mem::swap(&mut self.back_buffer, &mut self.front_buffer);
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
