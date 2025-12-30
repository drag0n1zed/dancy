use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

pub mod cartridge;
pub mod cpu;
pub mod io;
pub mod mmu;

// Is frame done? Check without locking Bus
pub type FrameSignal = Rc<Cell<bool>>;

pub struct EmulatorState {
    pub cpu: cpu::Cpu,
    pub bus: mmu::Bus,
}

// FFI entrypoint
pub struct DancyHandle {
    // Hardware state shared pointer
    state: Rc<RefCell<EmulatorState>>,
    // Persistent asynx execution loop
    execution_future: Pin<Box<dyn Future<Output = ()>>>,
    // Shared signal
    frame_ready: FrameSignal,
}

impl DancyHandle {
    pub fn new(rom_bytes: Vec<u8>) -> Self {
        let frame_ready = Rc::new(Cell::new(false));

        // Initialize Bus with signal
        let bus = mmu::Bus::new(rom_bytes, Rc::clone(&frame_ready));

        let state = Rc::new(RefCell::new(EmulatorState {
            cpu: cpu::Cpu::new(),
            bus,
        }));

        let state_for_future = Rc::clone(&state);
        let execution_future = Box::pin(async move {
            loop {
                // Borrow lives until instruction yields
                let mut s = state_for_future.borrow_mut();

                // SPLIT BORROW: see cpu and bus as separate fields
                let EmulatorState {
                    ref mut cpu,
                    ref mut bus,
                } = *s;

                cpu.step(bus).await;

                // lock dropped!
            }
        });

        Self {
            state,
            execution_future,
            frame_ready,
        }
    }

    // FFI should call this 60 times a second (or however many fps you want)
    pub fn run_frame(&mut self) {
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);

        loop {
            // Drive the CPU future
            match self.execution_future.as_mut().poll(&mut cx) {
                Poll::Pending => {
                    // Is frame done?
                    if self.frame_ready.get() {
                        self.frame_ready.set(false); // Reset signal
                        break; // Return to FFI
                    }
                }
                Poll::Ready(_) => break,
            }
        }
    }

    // Get buffer pixels
    pub fn get_pixels(&self) -> Vec<u8> {
        self.state.borrow().bus.ppu.front_buffer.to_vec()
    }

    // Update joypad state. 0 = pressed, `↓ ↑ ← → S s B A` as u8
    pub fn update_buttons(&mut self, pressed: u8) {
        self.state.borrow_mut().bus.joypad.set_buttons(pressed);
    }
}

fn dummy_waker() -> Waker {
    fn no_op(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cpu_instructions() {
        // Initialize env_logger
        let _logger = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        const TEST_ROMS: [&'static str; 11] = [
            "01-special",
            "02-interrupts",
            "03-op sp,hl",
            "04-op r,imm",
            "05-op rp",
            "06-ld r,r",
            "07-jr,jp,call,ret,rst",
            "08-misc instrs",
            "09-op r,r",
            "10-bit ops",
            "11-op a,(hl)",
        ];
        for rom_path in &TEST_ROMS {
            let full_rom_path = format!("test_roms/cpu_instrs/{}.gb", rom_path);
            let rom = std::fs::read(full_rom_path).unwrap();
            let mut handle = DancyHandle::new(rom);

            for _ in 0..1500 {
                handle.run_frame();
            }
            println!("-------------");
        }
    }

    #[test]
    fn test_instruction_timing() {
        // Initialize env_logger
        let _logger = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        let rom = std::fs::read("test_roms/instr_timing.gb").unwrap();
        let mut handle = DancyHandle::new(rom);

        for _ in 0..1500 {
            handle.run_frame();
        }
    }

    #[test]
    fn test_memory_timing() {
        // Initialize env_logger
        let _logger = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        let rom = std::fs::read("test_roms/mem_timing.gb").unwrap();
        let mut handle = DancyHandle::new(rom);

        for _ in 0..1500 {
            handle.run_frame();
        }
    }

    #[test]
    fn test_memory_timing_two() {
        // Initialize env_logger
        let _logger = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        let rom = std::fs::read("test_roms/mem_timing-2.gb").unwrap();
        let mut handle = DancyHandle::new(rom);

        for _ in 0..100000 {
            handle.run_frame();
        }
    }

    #[test]
    fn test_interrupt_timing() {
        // Initialize env_logger
        let _logger = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        let rom = std::fs::read("test_roms/interrupt_time.gb").unwrap();
        let mut handle = DancyHandle::new(rom);

        for _ in 0..1500 {
            handle.run_frame();
        }
    }

    #[test]
    fn test_halt_bug() {
        // Initialize env_logger
        let _logger = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        let rom = std::fs::read("test_roms/halt_bug.gb").unwrap();
        let mut handle = DancyHandle::new(rom);

        for _ in 0..10000 {
            handle.run_frame();
        }
    }
}
