pub struct Timer;

impl Timer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn step(&mut self, t_cycles: u32) {
        todo!();
    }

    pub fn read(&self, addr: u16) -> u8 {
        0
        // TODO
    }
    pub fn write(&mut self, addr: u16, value: u8) {
        // TODO
    }
}
