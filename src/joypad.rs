pub struct Joypad {
    state: u8,
    pub control: u8,
}
impl Joypad {
    pub fn new() -> Self {
        Self {
            state: 0xFF,
            control: 0x00,
        }
    }
    pub fn set_buttons(&mut self, bitmask: u8) {
        self.state = bitmask;
    }
    pub fn read(&self) -> u8 {
        0xFF // TODO: Connect IO
    }
    pub fn write(&mut self, value: u8) {
        // TODO
    }
}
