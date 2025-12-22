use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

mod cartridge;
mod cpu;
mod joypad;
mod mmu;
mod ppu;
mod timer;

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }
    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(std::ptr::null(), vtable)
}

fn main() {
    let rom_buffer: Vec<u8> = std::fs::read("tetris.gb").unwrap();
    let mut bus = mmu::Bus::new(rom_buffer);
    let mut cpu = cpu::Cpu::new();

    let waker = unsafe { Waker::from_raw(dummy_raw_waker()) }; // Is safe. Waker is No-Op
    let mut context = Context::from_waker(&waker);
    let mut cpu_future = Box::pin(cpu.step(&mut bus));
    loop {
        match cpu_future.as_mut().poll(&mut context) {
            Poll::Pending => {
                // Bus::tick() updates.
            }
            Poll::Ready(_) => {
                // finished
                break;
            }
        }
    }
}
