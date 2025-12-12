pub struct Joypad;
impl Joypad {
    pub fn new() -> Self {
        Self
    }
    pub fn read(&self) -> u8 {
        0xFF // TODO: Connect IO
    }
    pub fn write(&mut self, value: u8) {
        // TODO
    }
}
