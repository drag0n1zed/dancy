use crate::io::ppu::fetcher::{Fetcher, FetcherState};
use crate::io::ppu::pixel::{Pixel, PixelQueue};

mod pixel;
mod fetcher;

const PALETTE: [u32; 4] = [
    0xFFFFFFFF, // 0: White
    0xFFAAAAAA, // 1: Light Gray
    0xFF555555, // 2: Dark Gray
    0xFF000000, // 3: Black
];

#[derive(Default, Clone, Copy)]
struct Sprite {
    x: u8,
    y: u8,
    tile_index: u8,
    flags: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum PpuMode {
    HorizontalBlank,
    VerticalBlank,
    OAMScan,
    DrawingPixels,
}

pub struct Ppu {
    vram: [u8; 8192],
    oam: [u8; 160],

    r_lcdc: u8, // 0xFF40: Control
    r_stat: u8, // 0xFF41: Status & Interrupts
    r_scy: u8,  // 0xFF42: Scroll Y
    r_scx: u8,  // 0xFF43: Scroll X
    r_ly: u8,   // 0xFF44: Current scanline
    r_lyc: u8,  // 0xFF45: LY Compare
    r_bgp: u8,  // 0xFF47: Background Palette
    r_obp0: u8, // 0xFF48: Object Palette 0
    r_obp1: u8, // 0xFF49: Object Palette 1
    r_wy: u8,   // 0xFF4A: Window Y Position
    r_wx: u8,   // 0xFF4B: Window X Position

    dots: u32, // 0 - 455
    mode: PpuMode,
    lx: u8, // Current screen X pixel: 0 - 159

    bg_queue: PixelQueue,
    fetcher: Fetcher,
    sprite_buffer: [Sprite; 10],
    sprite_count: u8,
    discarded_pixels: u8,
    window_line_counter: u8,

    lcd_interrupt_signal: bool,
    back_buffer: [u32; 160 * 144],
    pub front_buffer: [u32; 160 * 144],
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            vram: [0; 8192],
            oam: [0; 160],

            r_lcdc: 0b1001_0001,
            r_stat: 0b1000_0101,
            r_scy: 0,
            r_scx: 0,
            r_ly: 0,
            r_lyc: 0,
            r_bgp: 0b1111_1100,
            r_obp0: 0b1111_1111,
            r_obp1: 0b1111_1111,
            r_wy: 0,
            r_wx: 0,

            dots: 0,
            mode: PpuMode::OAMScan,
            lx: 0,

            bg_queue: PixelQueue::new(),
            fetcher: Fetcher::new(),
            sprite_buffer: [Sprite::default(); 10],
            sprite_count: 0,
            discarded_pixels: 0,
            window_line_counter: 0,

            lcd_interrupt_signal: false,
            back_buffer: [0; 160 * 144],
            front_buffer: [0; 160 * 144],
        }
    }

    pub fn step(&mut self, t_cycles: u32) -> (bool, bool) {
        let mut vblank_irq = false;
        let mut lcd_stat_irq = false;
        for _ in 0..t_cycles {
            self.dots += 1;
            if self.r_ly == self.r_lyc {
                self.r_stat |= 0b0000_0100;
            } else {
                self.r_stat &= !0b0000_0100;
            }
            let signal = (self.r_stat & 0b0100_0000 != 0 && self.r_ly == self.r_lyc)
                || (self.r_stat & 0b0010_0000 != 0 && self.mode == PpuMode::OAMScan)
                || (self.r_stat & 0b0001_0000 != 0 && self.mode == PpuMode::VerticalBlank)
                || (self.r_stat & 0b0000_1000 != 0 && self.mode == PpuMode::HorizontalBlank);
            if signal && !self.lcd_interrupt_signal {
                lcd_stat_irq = true;
            }
            self.lcd_interrupt_signal = signal;

            match self.mode {
                PpuMode::OAMScan => {
                    if self.dots >= 80 {
                        self.fetcher.map_x = self.r_scx / 8;
                        self.fetcher.cycles = 0;
                        self.fetcher.state = FetcherState::GetTile;
                        self.fetcher.fetching_window = false;

                        self.bg_queue.clear();
                        // e.g. scx = 20, then we only see 4 pixels in Tile 2
                        self.discarded_pixels = self.r_scx % 8;
                        self.lx = 0;
                        self.mode = PpuMode::DrawingPixels;
                    }
                }
                PpuMode::DrawingPixels => {
                    let window_enable = (self.r_lcdc & 0x20) != 0;
                    let inside_window_y = self.r_ly >= self.r_wy;
                    let inside_window_x = (self.lx + 7) >= self.r_wx; // wx offset by 7

                    if window_enable && inside_window_y && inside_window_x {
                        self.fetcher.start_fetching_window(self.window_line_counter);
                    }
                    self.fetcher
                        .tick(&mut self.bg_queue, &self.vram, self.r_lcdc, self.r_scy, self.r_ly);
                    if let Some(pixel) = self.bg_queue.pop() {
                        if self.discarded_pixels > 0 {
                            self.discarded_pixels -= 1;
                        } else {
                            let color = self.resolve_pixel_color(pixel);
                            let index = (self.r_ly as usize * 160) + self.lx as usize; // 160 pixels per row
                            if index < self.back_buffer.len() {
                                self.back_buffer[index] = color;
                            }
                            self.lx += 1;
                        }
                    }
                    if self.lx >= 160 {
                        if self.fetcher.fetching_window {
                            self.window_line_counter += 1;
                        }
                        self.mode = PpuMode::HorizontalBlank;
                    }
                }
                PpuMode::HorizontalBlank => {
                    if self.dots >= 456 {
                        self.dots = 0;
                        self.r_ly += 1;
                        if self.r_ly == 144 {
                            vblank_irq = true;
                            self.window_line_counter = 0;
                            self.mode = PpuMode::VerticalBlank;
                        } else {
                            self.mode = PpuMode::OAMScan;
                        }
                    }
                }
                PpuMode::VerticalBlank => {
                    if self.dots >= 456 {
                        self.dots = 0;
                        self.r_ly += 1;
                        if self.r_ly > 153 {
                            self.r_ly = 0;
                            self.mode = PpuMode::OAMScan;
                        }
                    }
                }
            }
        }
        (vblank_irq, lcd_stat_irq)
    }

    fn resolve_pixel_color(&self, pixel: Pixel) -> u32 {
        let palette_reg = match pixel.palette_register {
            0 => self.r_bgp,
            1 => self.r_obp0,
            2 => self.r_obp1,
            _ => unreachable!(),
        };
        // e.g. if pixel.color is 3, bits 6-7.
        let color_bit = (palette_reg >> (pixel.color * 2)) & 0b11;
        PALETTE[color_bit as usize]
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
        match addr {
            0xFF40 => self.r_lcdc,
            0xFF41 => 0b1000_0000 | self.r_stat | (self.mode as u8),
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
            0xFF41 => self.r_stat = (self.r_stat & 0b0000_0111) | (value & 0b1111_1000),
            0xFF42 => self.r_scy = value,
            0xFF43 => self.r_scx = value,
            0xFF44 => {}
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
