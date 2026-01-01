#[derive(Default, Clone, Copy)]
pub(super) struct Sprite {
    pub(super) y: u8,
    pub(super) x: u8,
    pub(super) tile_index: u8,
    pub(super) attributes: u8,
}

impl Sprite{
    pub(super) fn new(y: u8, x: u8, tile_index: u8, attributes: u8) -> Self {
        Self {
            y,
            x,
            tile_index,
            attributes,
        }
    }
}