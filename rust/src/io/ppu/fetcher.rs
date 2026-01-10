#[repr(u8)]
enum ColorIndices {
    Zero,
    One,
    Two,
    Three,
}
struct Pixel {
    color: ColorIndices,
    uses_obp1: bool,
}
impl Pixel {
    fn new(color: ColorIndices, uses_obp1: bool) -> Self {
        Self { color, uses_obp1 }
    }
}

pub struct PixelFifo {
    queue: [Pixel; 16],
}
impl PixelFifo {
    pub fn new() -> Self {
        Self {
            queue: core::array::from_fn(|_| Pixel::new(ColorIndices::Zero, true)),
        }
    }
}

#[derive(PartialEq)]
enum FetcherModes {
    GetTileID,
    GetTileLow,
    GetTileHigh,
    Sleep,
    Push,
}
struct Fetcher {
    fifo: PixelFifo,
    mode: FetcherModes,
    tick: u32,
}
impl Fetcher {
    pub fn new() -> Self {
        Self {
            fifo: PixelFifo::new(),
            mode: FetcherModes::GetTileID,
            tick: 0,
        }
    }
    pub fn start(map_addr: u32, tile_row: u32) {}
    fn step(&mut self) {
        self.tick += 1;
        if self.tick < 2 && self.mode != FetcherModes::Push {
            return;
        }
        self.tick = 0;

        match self.mode {
            FetcherModes::GetTileID => {
                self.mode = FetcherModes::GetTileLow;
            }
            FetcherModes::GetTileLow => {
                self.mode = FetcherModes::GetTileHigh;
            }
            FetcherModes::GetTileHigh => {
                self.mode = FetcherModes::Sleep;
            }
            FetcherModes::Sleep => {
                self.mode = FetcherModes::Push;
            }
            FetcherModes::Push => {
                self.mode = FetcherModes::GetTileID;
            }
        }
    }
}
