pub struct Timer {
    timer_counter: u8,
    timer_modulo: u8,
    timer_control: u8,
    internal_counter: u16,
    interrupt_request: bool,
    cycles_until_tima_reload: u32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            // 0xFF04, DIV is internal_counter's high byte.
            // Tie internal_counter to t_cycles. 256 t-cycles is one DIV, so high byte is real DIV.
            timer_counter: 0x00, // 0xFF05, TIMA
            timer_modulo: 0x00,  // 0xFF06, TMA
            timer_control: 0x00, // 0xFF07, TAC
            internal_counter: 0,
            interrupt_request: false,
            cycles_until_tima_reload: 0,
        }
    }

    // Falling Edge Detector:
    // TIMA increments when target bit AND enable bit goes from 1 -> 0.
    // Things that trigger the increment:
    // 1. Every ? DIV increments (depending on TAC). Intended usecase.
    // 2. Target bit was 1, then DIV write reset it to 0. "Glitch".
    // 3. Enable bit was 1, but written to 0. "Glitch".

    pub fn step(&mut self, t_cycles: u32) -> bool {
        if self.cycles_until_tima_reload > 0 {
            if t_cycles >= self.cycles_until_tima_reload {
                self.cycles_until_tima_reload = 0;
                self.timer_counter = self.timer_modulo;
                self.interrupt_request = true;
            } else {
                self.cycles_until_tima_reload -= t_cycles;
            }
        }

        let signal_before = self.get_timer_signal();
        self.internal_counter = self.internal_counter.wrapping_add(t_cycles as u16);
        let signal_after = self.get_timer_signal();

        // If 1 -> 0
        if signal_before && !signal_after {
            self.inc_timer_counter(); // Interrupt if overflow
        }

        let interrupt_requested = self.interrupt_request;
        self.interrupt_request = false;
        interrupt_requested
    }

    fn get_timer_signal(&self) -> bool {
        let tac_enable_bit = self.timer_control & 0b0000_0100 != 0;
        let tac_bit_index = match self.timer_control & 0b0000_0011 {
            // 1024 t-cycles per inc -> 4096Hz -> bit 9 in DIV goes from 1 -> 0.
            // 0b0000_0011_1111_1111 -> 0b0000_0100_0000_0000. Easy formula: n = (log2 tcycles) - 1
            0x00 => 9,
            0x01 => 3, // 16 t-cycles
            0x10 => 5, // 64 t-cycles
            0x11 => 7, // 256 t-cycles
            _ => unreachable!(),
        };
        let tac_target_bit = self.internal_counter & (0b1 << tac_bit_index) != 0;
        tac_target_bit && tac_enable_bit
    }

    fn inc_timer_counter(&mut self) {
        let (new_val, overflow) = self.timer_counter.overflowing_add(1);
        if overflow {
            self.timer_counter = 0x00;
            self.cycles_until_tima_reload = 4;
        } else {
            self.timer_counter = new_val;
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => (self.internal_counter >> 8) as u8,
            0xFF05 => self.timer_counter,
            0xFF06 => self.timer_modulo,
            0xFF07 => self.timer_control | 0xF8, // Empty read as 1
            _ => unreachable!(),
        }
    }
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF04 => {
                let signal_before = self.get_timer_signal();
                self.internal_counter = 0;
                let signal_after = self.get_timer_signal();
                if signal_before && !signal_after {
                    self.inc_timer_counter();
                }
            }
            0xFF05 => {
                self.timer_counter = value;
                // TIMA reload canceled by TIMA write
                self.cycles_until_tima_reload = 0;
            }
            0xFF06 => self.timer_modulo = value,
            0xFF07 => {
                let signal_before = self.get_timer_signal();
                self.timer_control = value;
                let signal_after = self.get_timer_signal();
                if signal_before && !signal_after {
                    self.inc_timer_counter();
                }
            }
            _ => unreachable!(),
        }
    }
}
