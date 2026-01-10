mod fetcher;
use crate::SharedFrameBuffer;

enum Modes {
    HBlank,
    VBlank,
    OAMScan,
    Drawing,
}

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

    mode: Modes,
    dots: u32,
    lx: u32,
    tile_row: u32,
    tile_map_row_addr: u32,

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

            mode: Modes::OAMScan,
            dots: 0,
            lx: 0,
            tile_row: 0,          // Which row of the 8x8 tile is being read
            tile_map_row_addr: 0, // Address of the first tile in the row of 8x8 tiles on the map being read

            front_buffer: graphics_buffer,
        }
    }

    /// Returns (vblank_interrupt, stat_interrupt)
    pub fn step(&mut self, t_cycles: u32) -> (bool, bool) {
        let mut vblank_triggered = false;
        let mut lcd_stat_triggered = false;

        for _ in 0..t_cycles {
            self.dots += 1;

            match self.mode {
                Modes::OAMScan => {
                    if self.dots == 80 {
                        self.lx = 0;
                        self.tile_row = self.r_ly as u32 % 8;
                        self.mode = Modes::Drawing;
                    }
                }
                Modes::Drawing => {
                    // TODO: fetch pixel data into FIFO
                    // TODO: put a pixel from FIFO onto display
                    // TODO: Enter HBlank if scanline has all 160 pixels
                }
                Modes::HBlank => {
                    if self.dots >= 456 {
                        self.dots -= 456;
                        self.r_ly += 1;
                        if self.r_ly >= 144 {
                            self.mode = Modes::VBlank;
                        } else {
                            self.mode = Modes::OAMScan;
                        }
                    }
                }
                Modes::VBlank => {
                    if self.dots >= 456 {
                        self.dots -= 456;
                        self.r_ly += 1;
                        if self.r_ly >= 154 {
                            self.r_ly -= 154;
                            self.mode = Modes::OAMScan;
                        }
                    }
                }
            }
        }

        (vblank_triggered, lcd_stat_triggered)
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
            0xFF44 => { /* Read Only */ }
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
