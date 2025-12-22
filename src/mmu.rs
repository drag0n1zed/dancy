use crate::FrameSignal;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Instant;

use crate::cartridge::Cartridge;
use crate::joypad::Joypad;
use crate::ppu::Ppu;
use crate::timer::Timer;

struct Yield(bool);
impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub struct Bus {
    cartridge: Cartridge,
    pub ppu: Ppu,
    wram: [u8; 8192],
    hram: [u8; 127],
    pub timer: Timer,
    pub joypad: Joypad,
    pub interrupt_flag: u8,   // 0xFF0F
    pub interrupt_enable: u8, // 0xFFFF
    pub last_frame_time: Instant,
    pub accumulated_cycles: u32,
    pub frame_ready: FrameSignal,
}

impl Bus {
    pub fn new(rom_data: Vec<u8>, frame_ready: FrameSignal) -> Self {
        Bus {
            cartridge: Cartridge::new(rom_data),
            ppu: Ppu::new(),
            wram: [0; 8192],
            hram: [0; 127],
            timer: Timer::new(),
            joypad: Joypad::new(),
            interrupt_flag: 0,   // 0xFF0F
            interrupt_enable: 0, // 0xFFFF
            last_frame_time: Instant::now(),
            accumulated_cycles: 0,
            frame_ready,
        }
    }

    pub async fn tick(&mut self) {
        // 1. Step hardware
        self.ppu.step(4);
        self.timer.step(4);

        self.accumulated_cycles += 1;

        if self.accumulated_cycles >= 17556 {
            self.accumulated_cycles = 0;
            self.ppu.update_front_buffer();
            self.frame_ready.set(true);
            Yield(false).await;
        }
    }

    pub async fn read(&mut self, addr: u16) -> u8 {
        let value = self.raw_read(addr);
        self.tick().await;
        value
    }

    pub async fn write(&mut self, addr: u16, value: u8) {
        self.raw_write(addr, value);
        self.tick().await;
    }

    pub async fn read_u16(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr).await;
        let hi = self.read(addr.wrapping_add(1)).await;
        u16::from_le_bytes([lo, hi])
    }

    pub async fn write_u16(&mut self, addr: u16, value: u16, le: bool) {
        let [lo, hi] = value.to_le_bytes();
        if le {
            // LD (nn), SP
            self.write(addr, lo).await;
            self.write(addr.wrapping_add(1), hi).await;
        } else {
            // Everything else (stack operation)
            self.write(addr.wrapping_add(1), hi).await;
            self.write(addr, lo).await;
        }
    }

    pub fn raw_read(&self, addr: u16) -> u8 {
        match addr {
            // ROM bank 00
            0x0000..=0x3FFF => self.cartridge.read(addr),
            // ROM Bank 01–NN (switchable via MBCs)
            0x4000..=0x7FFF => self.cartridge.read(addr),
            // Video RAM (In CGB mode, switchable bank 0/1)
            0x8000..=0x9FFF => self.ppu.read_vram(addr),
            // External RAM (on cartridge, for save files)
            0xA000..=0xBFFF => self.cartridge.read(addr),
            // Work RAM
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            // Echo RAM
            0xE000..=0xFDFF => self.raw_read(addr - 0x2000),
            // Object Attribute Memory
            0xFE00..=0xFE9F => self.ppu.read_oam(addr),
            // Unusable memory
            0xFEA0..=0xFEFF => 0xFF,

            // IO Registers
            // Joypad Input
            0xFF00 => self.joypad.read(),
            // Timer Registers
            0xFF04..=0xFF07 => self.timer.read(addr),
            // Interrupt Flag Register
            0xFF0F => self.interrupt_flag,
            // PPU Registers
            0xFF40..=0xFF4B => self.ppu.read_register(addr),
            // TODO: CGB KEY1 0xFF4D - speed switch
            // TODO: APU Registers

            // High RAM
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            // Interrupt Enable Register
            0xFFFF => self.interrupt_enable,
            _ => unimplemented!("Unimplemented address {:04X}", addr),
        }
    }

    pub fn raw_write(&mut self, addr: u16, value: u8) {
        match addr {
            // Cartridge ROM
            0x0000..=0x7FFF => self.cartridge.write(addr, value),
            // Video RAM
            0x8000..=0x9FFF => self.ppu.write_vram(addr, value),
            // Cartridge Save Files
            0xA000..=0xBFFF => self.cartridge.write(addr, value),
            // Work RAM
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = value,
            // Echo RAM
            0xE000..=0xFDFF => self.raw_write(addr - 0x2000, value),
            // Object Attribute Memory
            0xFE00..=0xFE9F => self.ppu.write_oam(addr, value),
            // Unusable memory
            0xFEA0..=0xFEFF => {}

            // IO Registers
            // Joypad Input
            0xFF00 => self.joypad.write(value),
            // Timer Registers
            0xFF04..=0xFF07 => self.timer.write(addr, value),
            // Interrupt Flag Register
            0xFF0F => self.interrupt_flag = value,
            // LCD Control
            0xFF40..=0xFF4B => self.ppu.write_register(addr, value),

            // High RAM
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            // Interrupt Enable Register
            0xFFFF => self.interrupt_enable = value,
            _ => unimplemented!("Unimplemented address {:04X}", addr),
        }
    }
}
