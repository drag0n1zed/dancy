use super::pixel::{Pixel, PixelQueue};

pub(super) enum FetcherState {
    GetTile,
    GetDataLow,
    GetDataHigh,
    Push,
}

pub(super) struct Fetcher {
    pub(crate) state: FetcherState,
    pub(crate) cycles: u8,
    tile_index: u8,
    data_lo: u8,
    data_hi: u8,
    pub(crate) map_x: u8, // 0 - 31
    window_line_counter: u8,
    pub(crate) fetching_window: bool,
}
impl Fetcher {
    pub(super) fn new() -> Self {
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
    pub(super) fn tick(&mut self, queue: &mut PixelQueue, vram: &[u8], r_lcdc: u8, r_scy: u8, r_ly: u8) {
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
                    (r_ly.wrapping_add(r_scy)) % 8 // row position within tile
                };
                let addr = self.get_tile_data_address(self.tile_index, tile_row, r_lcdc);
                self.data_lo = vram[addr as usize];
                self.state = FetcherState::GetDataHigh;
            }
            FetcherState::GetDataHigh => {
                let tile_row = if self.fetching_window {
                    self.window_line_counter % 8
                } else {
                    (r_ly.wrapping_add(r_scy)) % 8 // row position within tile
                };
                let addr = self.get_tile_data_address(self.tile_index, tile_row, r_lcdc) + 1;
                self.data_hi = vram[addr as usize];
                self.state = FetcherState::Push;
            }
            FetcherState::Push => {
                if queue.len() > 8 {
                    return;
                }
                for i in (0..8).rev() {
                    let color = (((self.data_hi >> i) & 0b1) << 1) + ((self.data_lo >> i) & 0b1);
                    let pixel = Pixel::new(color, 0, false, false);
                    if !queue.push(pixel) {
                        unreachable!();
                    }
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

    pub(super) fn start_fetching_window(&mut self, window_line: u8) {
        self.fetching_window = true;
        self.state = FetcherState::GetTile;
        self.map_x = 0;
        self.window_line_counter = window_line;
    }
}