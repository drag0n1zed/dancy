use crate::io::ppu::fetcher::{Fetcher, FetcherState};
use crate::io::ppu::pixel::{Pixel, PixelQueue};
use crate::io::ppu::sprite::Sprite;

mod fetcher;
mod pixel;
mod sprite;

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (0xFF << 24) | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32)
}
const PALETTE: [u32; 4] = [
    rgb(255, 255, 255), // 0: White
    rgb(170, 170, 170), // 1: Light Gray
    rgb(85,  85,  85),  // 2: Dark Gray
    rgb(0,   0,   0),   // 3: Black
];

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
    sprite_line_buffer: [Option<Pixel>; 160],
    sprite_count: u8,
    discarded_pixels: u8,
    window_line_counter: u8,

    lcd_interrupt_signal: bool,
    pub graphics_buffer: [u32; 160 * 144],
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
            sprite_line_buffer: [None; 160],
            sprite_count: 0,
            discarded_pixels: 0,
            window_line_counter: 0,

            lcd_interrupt_signal: false,
            graphics_buffer: [0; 160 * 144],
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
                        // Scan logic
                        self.sprite_count = 0;
                        self.sprite_buffer = [Sprite::default(); 10];
                        let sprite_height = if self.r_lcdc & 0b0000_0100 != 0 { 16 } else { 8 }; // 8x8 or 8x16
                        for i in 0..40 {
                            let y_pos = self.oam[i * 4];

                            if (self.r_ly as u16 + 16) >= (y_pos as u16)
                                && (self.r_ly as u16) < (y_pos as u16 + 16 + sprite_height as u16)
                            {
                                let x_pos = self.oam[i * 4 + 1];
                                let tile_index = self.oam[i * 4 + 2];
                                let attributes = self.oam[i * 4 + 3];

                                let sprite = Sprite::new(y_pos, x_pos, tile_index, attributes);
                                self.sprite_buffer[self.sprite_count as usize] = sprite;
                                self.sprite_count += 1;
                                if self.sprite_count == 10 {
                                    break;
                                }
                            }
                        }
                        // Sort by x, tiebreak with OAM (Already sorted in OAM order, so stable sort by x to get desired result)
                        self.sprite_buffer[0..self.sprite_count as usize].sort_by(|a, b| a.x.cmp(&b.x));
                        self.load_line_sprites();

                        // Start render
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

                    if let Some(mut bg_pixel) = self.bg_queue.pop() {
                        if self.r_lcdc & 0b0000_0001 == 0 {
                            bg_pixel.color = 0;
                        }

                        if self.discarded_pixels > 0 {
                            self.discarded_pixels -= 1;
                        } else {
                            let sprite_pixel = self.sprite_line_buffer[self.lx as usize];
                            let final_pixel = if let Some(sprite) = sprite_pixel {
                                if sprite.bg_priority && bg_pixel.color != 0 {
                                    // Sprite behind BG, BG not transparent
                                    bg_pixel
                                } else {
                                    // Sprite on top / BG is transparent
                                    sprite
                                }
                            } else {
                                // No sprite, BG win
                                bg_pixel
                            };

                            let color = self.resolve_pixel_color(final_pixel);
                            let index = (self.r_ly as usize * 160) + self.lx as usize; // 160 pixels per row
                            if index < self.graphics_buffer.len() {
                                self.graphics_buffer[index] = color;
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
    fn load_line_sprites(&mut self) {
        self.sprite_line_buffer = [None; 160];
        let sprite_height = if self.r_lcdc & 0b0000_0100 != 0 { 16 } else { 8 };

        for i in 0..self.sprite_count as usize {
            let sprite = self.sprite_buffer[i];

            // which row?
            let mut row = (self.r_ly as u16 + 16).wrapping_sub(sprite.y as u16) as u8;

            // y-flip
            if sprite.attributes & 0x40 != 0 {
                row = sprite_height - 1 - row;
            }

            let mut tile_index = sprite.tile_index;
            if sprite_height == 16 {
                // In 8x16 mode, the top tile is even (index & 0xFE), bottom is odd.
                tile_index &= 0xFE;
                if row >= 8 {
                    tile_index += 1;
                    row -= 8;
                }
            }

            let addr = 0x8000 + (tile_index as u16 * 16) + (row as u16 * 2);
            let data_lo = self.vram[(addr - 0x8000) as usize];
            let data_hi = self.vram[(addr - 0x8000 + 1) as usize];

            // Iterate through the 8 pixels
            for bit in 0..8 {
                let pixel_x = (sprite.x as i16 - 8) + bit as i16;

                // Skip if pixel is off-screen
                if pixel_x < 0 || pixel_x >= 160 {
                    continue;
                }

                // Handle X-Flip (Attribute Bit 5)
                // If flipped, we read bits 0..7. If normal, we read 7..0.
                let shift = if sprite.attributes & 0x20 != 0 {
                    bit // Read from right (LSB)
                } else {
                    7 - bit // Read from left (MSB)
                };

                // Extract color
                let color = (((data_hi >> shift) & 1) << 1) | ((data_lo >> shift) & 1);

                // 0 is always transparent
                if color == 0 {
                    continue;
                }

                // priority
                if self.sprite_line_buffer[pixel_x as usize].is_none() {
                    self.sprite_line_buffer[pixel_x as usize] = Some(Pixel::new(
                        color,
                        1 + ((sprite.attributes >> 4) & 1), // Bit 4: 0=OBP0, 1=OBP1, add 1 to match my definition
                        (sprite.attributes & 0x80) != 0,    // Bit 7: Priority
                        true,
                    ))
                }
            }
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
