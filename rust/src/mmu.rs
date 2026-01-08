use crate::{FrameSignal, SharedFrameBuffer};
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Instant;

use crate::cartridge::Cartridge;
use crate::io::apu::Apu;
use crate::io::joypad::Joypad;
use crate::io::{ppu::Ppu, serial::Serial, timer::Timer};

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
    wram: [u8; 8192],
    hram: [u8; 127],

    pub ppu: Ppu,
    pub apu: Apu,

    pub timer: Timer,
    pub joypad: Joypad,
    pub serial: Serial,

    dma_active: bool,
    dma_base: u8,
    dma_byte: u8,
    dma_delay: u8,

    pub interrupt_flag: u8,   // 0xFF0F
    pub interrupt_enable: u8, // 0xFFFF

    pub last_frame_time: Instant,
    pub accumulated_cycles: u32,

    pub frame_ready: FrameSignal,
}

impl Bus {
    pub fn new(rom_data: Vec<u8>, frame_ready: FrameSignal, video_buffer: SharedFrameBuffer) -> Self {
        Bus {
            cartridge: Cartridge::new(rom_data),
            wram: [0; 8192],
            hram: [0; 127],

            ppu: Ppu::new(video_buffer),
            apu: Apu::new(),

            timer: Timer::new(),
            joypad: Joypad::new(),
            serial: Serial::new(),

            dma_active: false,
            dma_base: 0,
            dma_byte: 0,
            dma_delay: 0,

            interrupt_flag: 0x00,
            interrupt_enable: 0x00,

            last_frame_time: Instant::now(),
            accumulated_cycles: 0,

            frame_ready,
        }
    }

    pub async fn tick(&mut self) {
        // DMA
        if self.dma_active {
            if self.dma_delay > 0 {
                self.dma_delay -= 1;
            } else {
                let src_addr = ((self.dma_base as u16) << 8) + (self.dma_byte as u16);
                let byte = self.unblocked_raw_read(src_addr);
                self.ppu.write_oam(0xFE00 + (self.dma_byte as u16), byte);
                self.dma_byte = self.dma_byte.wrapping_add(1);
                if self.dma_byte >= 160 {
                    self.dma_active = false;
                }
            }
        }

        // Step hardware
        let (v_blank, lcd_stat) = self.ppu.step(4);
        if v_blank {
            self.interrupt_flag |= 0b0000_0001;

            self.frame_ready.set(true); // Signal frame ready
            Yield(false).await;
        }
        if lcd_stat {
            self.interrupt_flag |= 0b0000_0010;
        }
        if self.timer.step(4) {
            self.interrupt_flag |= 0b0000_0100;
        }
        if self.serial.step(4) {
            self.interrupt_flag |= 0b0000_1000;
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
        if self.dma_active {
            if addr < 0xFF80 || addr > 0xFFFE {
                return 0xFF;
            }
        }
        self.unblocked_raw_read(addr)
    }

    fn unblocked_raw_read(&self, addr: u16) -> u8 {
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
            0xE000..=0xFDFF => self.wram[((addr - 0x2000) - 0xC000) as usize],
            // Object Attribute Memory
            0xFE00..=0xFE9F => self.ppu.read_oam(addr),
            // Unusable memory
            0xFEA0..=0xFEFF => 0xFF,
            // IO Registers
            0xFF00..=0xFF7F => self.read_io(addr),
            // High RAM
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            // Interrupt Enable Register
            0xFFFF => self.interrupt_enable,
        }
    }

    pub fn raw_write(&mut self, addr: u16, value: u8) {
        if self.dma_active {
            if addr < 0xFF80 || addr > 0xFFFE {
                return;
            }
        }
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
            0xFF00..=0xFF7F => self.write_io(addr, value),
            // High RAM
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            // Interrupt Enable Register
            0xFFFF => self.interrupt_enable = value,
        }
    }

    fn read_io(&self, addr: u16) -> u8 {
        match addr {
            // Joypad Input
            0xFF00 => self.joypad.read(),
            // Serial Transfer
            0xFF01..=0xFF02 => self.serial.read(addr),
            // Timer and Divider
            0xFF04..=0xFF07 => self.timer.read(addr),
            // Interrupt Flag Register
            0xFF0F => self.interrupt_flag | 0xE0,
            // Audio
            0xFF10..=0xFF26 => self.apu.read(addr),
            // DMA
            0xFF46 => self.dma_base,
            // LCD Control, Status, Position, Scrolling, and Palettes
            0xFF40..=0xFF4B => self.ppu.read_register(addr),
            // CGB
            0xFF4C | 0xFF4D | 0xFF4F | 0xFF51..=0xFF55 | 0xFF56 | 0xFF68..=0xFF6B | 0xFF6C | 0xFF70 => {
                self.read_cgb_io(addr)
            }
            _ => {
                println!("Unimplemented read to IO address {:04X}", addr);
                0xFF
            }
        }
    }

    fn write_io(&mut self, addr: u16, value: u8) {
        match addr {
            // Joypad Input
            0xFF00 => self.joypad.write(value),
            // Serial Transfer
            0xFF01..=0xFF02 => self.serial.write(addr, value),
            // Timer and Divider
            0xFF04..=0xFF07 => self.timer.write(addr, value),
            // Interrupt Flag Register
            0xFF0F => self.interrupt_flag = value,
            // Audio
            0xFF10..=0xFF26 => self.apu.write(addr, value),
            // DMA
            0xFF46 => {
                self.dma_active = true;
                self.dma_base = value;
                self.dma_byte = 0;
                self.dma_delay = 2;
            }
            // LCD Control, Status, Position, Scrolling, and Palettes
            0xFF40..=0xFF4B => self.ppu.write_register(addr, value),
            // CGB KEY1 Double Speed
            0xFF4C | 0xFF4D | 0xFF4F | 0xFF51..=0xFF55 | 0xFF56 | 0xFF68..=0xFF6B | 0xFF6C | 0xFF70 => {
                self.write_cgb_io(addr, value)
            }
            _ => println!("Unimplemented write to IO address {:04X}", addr),
        }
    }

    fn read_cgb_io(&self, _addr: u16) -> u8 {
        0x00 // TODO: CGB
    }

    fn write_cgb_io(&self, _addr: u16, _value: u8) {
        // TODO: CGB
    }
}
