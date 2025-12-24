pub struct Joypad {
    pressed: u8,
    inverse_select_action: bool,
    inverse_select_dpad: bool,
}
impl Joypad {
    pub fn new() -> Self {
        Self {
            pressed: 0b1111_1111,        // 0 is set, no buttons pressed by default
            inverse_select_action: true, // true for NOT SET
            inverse_select_dpad: true,
        }
    }
    pub fn set_buttons(&mut self, pressed: u8) {
        self.pressed = pressed;
    }
    pub fn read(&self) -> u8 {
        let mut lo = 0b0000_1111;
        if !self.inverse_select_action {
            // Action, SsBA
            lo &= self.pressed & 0x0F;
        }
        if !self.inverse_select_dpad {
            // D-Pad, DULR
            lo &= (self.pressed >> 4) & 0x0F;
        }
        // lo is 0b0000_xxxx
        let hi = 0b1100_0000
            | (if self.inverse_select_action { 1 } else { 0 } << 5)
            | (if self.inverse_select_dpad { 1 } else { 0 } << 4);
        // hi is 0b11xx_0000
        hi | lo
        // hi | lo = 0b11xx_xxxx
    }
    pub fn write(&mut self, value: u8) {
        self.inverse_select_action = value & 0b0010_0000 != 0;
        self.inverse_select_dpad = value & 0b0001_0000 != 0;
    }
}
