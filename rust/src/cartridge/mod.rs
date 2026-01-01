mod mbc;
use crate::cartridge::mbc::{Mbc, mbc0::Mbc0, mbc1::Mbc1};

pub struct Cartridge {
    pub mbc: Box<dyn Mbc>,
}

impl Cartridge {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let mbc_type = rom_data[0x0147];
        let mbc: Box<dyn Mbc> = match mbc_type {
            0x00 => Box::new(Mbc0::new(rom_data)),        // MBC0
            0x01 => Box::new(Mbc1::new(rom_data, false)), // MBC1
            0x02 => Box::new(Mbc1::new(rom_data, false)), // MBC1 + RAM
            0x03 => Box::new(Mbc1::new(rom_data, true)),  // MBC1 + RAM + BATTERY
            0x08 | 0x09 => panic!("Unsupported MBC0 + RAM"),
            _ => panic!("Unimplemented MBC type"),
        };
        Self { mbc }
    }
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.mbc.read_rom(addr),
            0xA000..=0xBFFF => self.mbc.read_ram(addr),
            _ => unreachable!(),
        }
    }
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.mbc.write_rom(addr, value),
            0xA000..=0xBFFF => self.mbc.write_ram(addr, value),
            _ => unreachable!(),
        }
    }
}
