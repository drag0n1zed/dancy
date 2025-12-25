use log::info;

pub struct Serial {
    pub sb: u8,      // 0xFF01
    pub sc: u8,      // 0xFF02
    timer: u32,      // Cycle count (0 to 511)
    shift_count: u8, // Bit count (0 to 8)
    current_char: u8,
    pub log_buffer: String,
}
impl Serial {
    pub fn new() -> Self {
        Self {
            sb: 0x00,
            sc: 0x00,
            timer: 0,
            shift_count: 0,
            current_char: 0,
            log_buffer: String::new(),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF01 => self.sb,
            0xFF02 => self.sc | 0x7E, // Unused bits read as 1
            _ => unreachable!(),
        }
    }
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF01 => {
                self.sb = value;
            }
            0xFF02 => {
                self.sc = value;
                if (value & 0b1000_0001) == 0b1000_0001 {
                    self.current_char = self.sb;
                    self.timer = 4194304 / 8192; // 512
                    self.shift_count = 0;
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn step(&mut self, t_cycles: u32) -> bool {
        if self.sc & 0x80 == 0 {
            return false; // Transfer inactive
        }

        self.timer = self.timer.saturating_sub(t_cycles);

        // If transfer complete
        if self.timer == 0 {
            let _out_bit = self.sb & 0b1000_0000;
            let in_bit = 0b0000_0001;
            self.sb = (self.sb << 1) | in_bit;
            self.shift_count += 1;

            if self.shift_count == 8 {
                // 8 bits to shift
                self.sc &= 0b0111_1111; // Clear flag

                // Log Output (for testing)
                let c = self.current_char as char;
                if c == '\n' {
                    info!(target: "gb_serial", "{}", self.log_buffer);
                    self.log_buffer.clear();
                } else {
                    self.log_buffer.push(c);
                }

                return true;
            } else {
                self.timer = 512;
            }
        }
        false
    }
}
