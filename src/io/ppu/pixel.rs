#[derive(Clone, Copy, Default)]
pub(super) struct Pixel {
    pub(super) color: u8,
    pub(super) palette_register: u8, // 0 -> BGP, 1 -> OBP0, 2 -> OBP1
    bg_priority: bool,    // true -> background will draw over this pixel
    is_sprite: bool,
}
impl Pixel{
    pub(super) fn new(color: u8, palette_register: u8, bg_priority: bool, is_sprite: bool) -> Self {
        Self {
            color,
            palette_register,
            bg_priority,
            is_sprite,
        }
    }
}

// custom static length queue of 16 Pixels
pub(super) struct PixelQueue {
    data: [Pixel; 16],
    head: usize,
    tail: usize,
    len: usize,
}
impl PixelQueue {
    pub(super) fn new() -> Self {
        Self {
            data: [Pixel::default(); 16],
            head: 0,
            tail: 0,
            len: 0,
        }
    }
    pub(super) fn push(&mut self, pixel: Pixel) -> bool {
        if self.len == 16 {
            false
        } else {
            self.data[self.tail] = pixel;
            self.tail = (self.tail + 1) % 16;
            self.len += 1;
            true
        }
    }
    pub(super) fn pop(&mut self) -> Option<Pixel> {
        if self.len == 0 {
            None
        } else {
            let pixel = self.data[self.head];
            self.head = (self.head + 1) % 16;
            self.len -= 1;
            Some(pixel)
        }
    }
    pub(super) fn len(&self) -> usize {
        self.len
    }
    pub(super) fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.len = 0;
    }
}