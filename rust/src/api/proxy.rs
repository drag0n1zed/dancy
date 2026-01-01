use std::ops::Deref;
use crate::DancyHandle;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use flutter_rust_bridge::frb;
use parking_lot::Mutex;

enum EmuCommand {
    Tick,
    UpdateButtons(u8),
}

#[frb(opaque)]
pub struct DancyProxy {
    tx: Sender<EmuCommand>,
    frame_rx: Mutex<Receiver<Vec<u8>>>,
}

impl DancyProxy {
    pub fn new(rom_bytes: Vec<u8>) -> DancyProxy {
        let (cmd_tx, cmd_rx) = channel::<EmuCommand>();
        let (frame_tx, frame_rx) = channel::<Vec<u8>>();

        thread::spawn(move || {
            let mut emulator = DancyHandle::new(rom_bytes);

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    EmuCommand::Tick => {
                        emulator.run_frame();
                        let pixels = emulator.get_graphics();
                        let _ = frame_tx.send(pixels);
                    }
                    EmuCommand::UpdateButtons(state) => {
                        emulator.update_buttons(state);
                    }
                }
            }
        });

        DancyProxy {
            tx: cmd_tx,
            frame_rx: Mutex::new(frame_rx),
        }
    }

    pub fn tick(&self) -> Vec<u8> {
        // Send command
        self.tx.send(EmuCommand::Tick).unwrap();
        self.frame_rx.lock().deref().recv().unwrap_or(vec![])
    }

    pub fn set_buttons(&self, pressed: u8) {
        let _ = self.tx.send(EmuCommand::UpdateButtons(pressed));
    }
}