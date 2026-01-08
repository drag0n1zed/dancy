use crate::SharedFrameBuffer;

pub struct Ppu {
    vram: [u8; 8192],
    oam: [u8; 160],

    r_lcdc: u8, // 0xFF40: LCD Control
    r_stat: u8, // 0xFF41: LCD Status
    r_scy: u8,  // 0xFF42: Scroll Y
    r_scx: u8,  // 0xFF43: Scroll X
    r_ly: u8,   // 0xFF44: LCD Y-Coordinate (Read Only)
    r_lyc: u8,  // 0xFF45: LY Compare
    r_bgp: u8,  // 0xFF47: BG Palette Data
    r_obp0: u8, // 0xFF48: Object Palette 0
    r_obp1: u8, // 0xFF49: Object Palette 1
    r_wy: u8,   // 0xFF4A: Window Y Position
    r_wx: u8,   // 0xFF4B: Window X Position

    pub dots: u32,
    pub front_buffer: SharedFrameBuffer,
}

impl Ppu {
    pub fn new(graphics_buffer: SharedFrameBuffer) -> Self {
        Self {
            vram: [0; 8192],
            oam: [0; 160],

            r_lcdc: 0x91,
            r_stat: 0x85,
            r_scy: 0,
            r_scx: 0,
            r_ly: 0,
            r_lyc: 0,
            r_bgp: 0xFC,
            r_obp0: 0xFF,
            r_obp1: 0xFF,
            r_wy: 0,
            r_wx: 0,

            dots: 0,
            front_buffer: graphics_buffer,
        }
    }

    /// Returns (vblank_interrupt, stat_interrupt)
    pub fn step(&mut self, t_cycles: u32) -> (bool, bool) {
        // TODO: Implement your scanline timing logic here
        // 1. Update self.dots
        // 2. Handle LY increments (every 456 dots)
        // 3. Handle Mode transitions (OAM Scan, Drawing, HBlank, VBlank)
        // 4. Trigger render_scanline() at appropriate mode

        (false, false)
    }


    pub fn read_vram(&self, addr: u16) -> u8 {
        self.vram[(addr & 0x1FFF) as usize]
    }

    pub fn write_vram(&mut self, addr: u16, value: u8) {
        self.vram[(addr & 0x1FFF) as usize] = value;
    }

    pub fn read_oam(&self, addr: u16) -> u8 {
        self.oam[(addr & 0xFF) as usize]
    }

    pub fn write_oam(&mut self, addr: u16, value: u8) {
        self.oam[(addr & 0xFF) as usize] = value;
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.r_lcdc,
            0xFF41 => self.r_stat | 0x80,
            0xFF42 => self.r_scy,
            0xFF43 => self.r_scx,
            0xFF44 => self.r_ly,
            0xFF45 => self.r_lyc,
            0xFF47 => self.r_bgp,
            0xFF48 => self.r_obp0,
            0xFF49 => self.r_obp1,
            0xFF4A => self.r_wy,
            0xFF4B => self.r_wx,
            _ => 0xFF,
        }
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF40 => self.r_lcdc = value,
            0xFF41 => self.r_stat = (value & 0xF8) | (self.r_stat & 0x07),
            0xFF42 => self.r_scy = value,
            0xFF43 => self.r_scx = value,
            0xFF44 => { /* Read Only */ },
            0xFF45 => self.r_lyc = value,
            0xFF47 => self.r_bgp = value,
            0xFF48 => self.r_obp0 = value,
            0xFF49 => self.r_obp1 = value,
            0xFF4A => self.r_wy = value,
            0xFF4B => self.r_wx = value,
            _ => {}
        }
    }
}