use super::Mbc;

pub struct Mbc1 {
    rom: Vec<u8>,
    num_of_rom_banks: usize,
    ram: Vec<u8>,
    num_of_ram_banks: usize,
    ram_enabled: bool,
    bank_reg_1: u8,
    bank_reg_2: u8,
    advanced_mode: bool,
    ram_persistent: bool,
}

impl Mbc1 {
    pub fn new(rom_data: Vec<u8>, ram_persistent: bool) -> Self {
        let num_of_rom_banks = 0b1 << (rom_data[0x0148] + 1);
        let num_of_ram_banks = match rom_data[0x0149] {
            0x00 => 0,
            0x02 => 1,
            0x03 => 4,
            0x04 => 16,
            0x05 => 8,
            _ => panic!("Unsupported RAM size"),
        };
        Mbc1 {
            rom: rom_data,
            num_of_rom_banks,
            ram: vec![0; num_of_ram_banks * 0x2000],
            num_of_ram_banks,
            ram_enabled: false,
            bank_reg_1: 0x01, // 5 bits used
            bank_reg_2: 0x00, // 2 bits used
            advanced_mode: false,
            ram_persistent,
        }
    }
}

impl Mbc for Mbc1 {
    fn read_rom(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => {
                let target_bank = if self.advanced_mode {
                    (self.bank_reg_2 << 5) as usize % self.num_of_rom_banks
                } else {
                    0
                };
                self.rom[target_bank * 0x4000 + addr as usize]
            }
            0x4000..=0x7FFF => {
                let target_bank = ((self.bank_reg_2 << 5) | self.bank_reg_1) as usize % self.num_of_rom_banks;
                self.rom[target_bank * 0x4000 + (addr as usize - 0x4000)]
            }
            _ => unreachable!(),
        }
    }

    fn write_rom(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x3FFF => self.bank_reg_1 = if value & 0x1F == 0 { 0x01 } else { value & 0x1F },
            0x4000..=0x5FFF => self.bank_reg_2 = value & 0x03,
            0x6000..=0x7FFF => self.advanced_mode = value & 0x01 != 0x00,
            _ => unreachable!(),
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if !self.ram_enabled || self.num_of_ram_banks == 0 {
            return 0xFF;
        }
        let bank = if self.advanced_mode {
            self.bank_reg_2 as usize % self.num_of_ram_banks
        } else {
            0
        };

        self.ram[bank * 0x2000 + (addr as usize - 0xA000)]
    }

    fn write_ram(&mut self, addr: u16, value: u8) {
        if !self.ram_enabled || self.num_of_ram_banks == 0 {
            return;
        }
        let bank = if self.advanced_mode {
            self.bank_reg_2 as usize % self.num_of_ram_banks
        } else {
            0
        };

        self.ram[bank * 0x2000 + (addr as usize - 0xA000)] = value;
    }
}
