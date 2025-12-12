mod mmu;
mod cpu;
mod cartridge;
mod ppu;
mod timer;
mod joypad;

fn main() {
    let rom_buffer: Vec<u8> = std::fs::read("tetris.gb").unwrap();
    
    let mut bus = mmu::Bus::new(rom_buffer);
    
    let mut cpu = cpu::Cpu::new();
    
    loop {
        let cycles_this_step = cpu.step(&mut bus);
    }
}
