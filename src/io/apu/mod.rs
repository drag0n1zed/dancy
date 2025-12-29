pub struct Apu;

impl Apu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn read(&self, addr: u16) -> u8 {
        0xFF // TODO
    }
    
    pub fn write(&mut self, addr: u16, value: u8) {
        // TODO
    }
}