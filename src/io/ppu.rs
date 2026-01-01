const PALETTE: [u32; 4] = [
    0xFFFFFFFF, // 0: White
    0xFFAAAAAA, // 1: Light Gray
    0xFF555555, // 2: Dark Gray
    0xFF000000, // 3: Black
];

#[derive(Clone, Copy, Default)]
struct Pixel {
    color: u8,
    palette_register: u8, // 0 -> BGP, 1 -> OBP0, 2 -> OBP1
    bg_priority: bool,    // true -> background will draw over this pixel
    is_sprite: bool,
}

// custom static length queue of 16 Pixels
struct PixelQueue {
    data: [Pixel; 16],
    head: usize,
    tail: usize,
    len: usize,
}
impl PixelQueue {
    fn new() -> Self {
        Self {
            data: [Pixel::default(); 16],
            head: 0,
            tail: 0,
            len: 0,
        }
    }
    fn push(&mut self, pixel: Pixel) -> bool {
        if self.len == 16 {
            false
        } else {
            self.data[self.tail] = pixel;
            self.tail = (self.tail + 1) % 16;
            self.len += 1;
            true
        }
    }
    fn pop(&mut self) -> Option<Pixel> {
        if self.len == 0 {
            None
        } else {
            let pixel = self.data[self.head];
            self.head = (self.head + 1) % 16;
            self.len -= 1;
            Some(pixel)
        }
    }
    fn len(&self) -> usize {
        self.len
    }
    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.len = 0;
    }
}

enum FetcherState {
    GetTile,
    GetDataLow,
    GetDataHigh,
    Push,
}

struct Fetcher {
    state: FetcherState,
    cycles: u8,
    tile_index: u8,
    data_lo: u8,
    data_hi: u8,
    map_x: u8, // 0 - 31
    window_line_counter: u8,
    fetching_window: bool,
}
impl Fetcher {
    fn new() -> Self {
        Self {
            state: FetcherState::GetTile,
            cycles: 0,
            tile_index: 0,
            data_lo: 0,
            data_hi: 0,
            map_x: 0,
            window_line_counter: 0,
            fetching_window: false,
        }
    }
    fn tick(&mut self, queue: &mut PixelQueue, vram: &[u8], r_lcdc: u8, r_scy: u8, r_ly: u8) {
        self.cycles += 1;
        if self.cycles < 2 {
            return;
        } else {
            self.cycles = 0;
        }

        match self.state {
            FetcherState::GetTile => {
                let map_y = if self.fetching_window {
                    self.window_line_counter / 8
                } else {
                    ((r_ly.wrapping_add(r_scy)) / 8) % 32
                }; // tile position within map
                let addr = self.get_bg_map_address(self.map_x, map_y, r_lcdc);
                self.tile_index = vram[addr as usize];
                self.state = FetcherState::GetDataLow;
            }
            FetcherState::GetDataLow => {
                let tile_row = if self.fetching_window {
                    self.window_line_counter % 8
                } else {
                    (r_ly + r_scy) % 8 // row position within tile
                };
                let addr = self.get_tile_data_address(self.tile_index, tile_row, r_lcdc);
                self.data_lo = vram[addr as usize];
                self.state = FetcherState::GetDataHigh;
            }
            FetcherState::GetDataHigh => {
                let tile_row = if self.fetching_window {
                    self.window_line_counter % 8
                } else {
                    (r_ly + r_scy) % 8 // row position within tile
                };
                let addr = self.get_tile_data_address(self.tile_index, tile_row, r_lcdc) + 1;
                self.data_hi = vram[addr as usize];
                self.state = FetcherState::Push;
            }
            FetcherState::Push => {
                if queue.len() >= 8 {
                    return;
                }
                for i in 8..0 {
                    let color = (((self.data_hi >> i) & 0b1) << 1) + ((self.data_lo >> i) & 0b1);
                    let pixel = Pixel {
                        color,
                        palette_register: 0,
                        bg_priority: false,
                        is_sprite: false,
                    };
                    queue.push(pixel);
                }
                self.map_x = (self.map_x + 1) % 32;
                self.state = FetcherState::GetTile;
            }
        }
    }

    // The Background Map is two fixed grids of 32x32 tiles
    // at 0x9C00 and 0x9800 depending on the LCDC register.
    // To find a tile at (x, y), we calculate the index using y * 32 + x.
    fn get_bg_map_address(&self, map_x: u8, map_y: u8, r_lcdc: u8) -> u16 {
        let window_or_bg_bit = if self.fetching_window { 0b0100_0000 } else { 0b0000_1000 };
        let base = if r_lcdc & window_or_bg_bit != 0 { 0x9C00 } else { 0x9800 };
        let addr = base + (map_y as u16) * 32 + map_x as u16;
        addr - 0x8000
    }

    // The Tile Data is stored in two fixed grids of 8x8 tiles.
    // One row is 8 pixels, every pixel is 2 bits, so one row is 2 bytes.
    // Unsigned mode:
    // Tile 0 at 0x8000, tile 1 at 0x8010, etc.
    // Signed mode (Shared VRAM):
    // Tile 0 to 127 at 0x9000 -> 0x97F0
    // Tile 128 - 255 are considered as -128 to -1, at 0x8800 -> 0x8FF0
    fn get_tile_data_address(&self, tile_index: u8, row: u8, r_lcdc: u8) -> u16 {
        let addr = if r_lcdc & 0b0001_0000 != 0 {
            0x8000 + (tile_index as u16 * 16) + (row as u16 * 2)
        } else {
            0x9000_u16.wrapping_add_signed(tile_index as i8 as i16 * 16) + (row as u16 * 2)
        };
        addr - 0x8000
    }

    fn start_fetching_window(&mut self, window_line: u8) {
        self.fetching_window = true;
        self.state = FetcherState::GetTile;
        self.map_x = 0;
        self.window_line_counter = window_line;
    }
}

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
