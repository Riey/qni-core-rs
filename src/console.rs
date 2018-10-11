use bus::{Bus, BusReader};
use crate::protos::qni_api::*;
use multiqueue::*;
use protobuf::{Message, RepeatedField};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::{Mutex, RwLock};
use std::thread;
use std::time::Duration;

pub struct ConsoleContext {
    commands: RwLock<Vec<ProgramCommand>>,
    send_bus: Mutex<Bus<Vec<u8>>>,
    response_tx: Mutex<MPMCSender<ConsoleResponse>>,
    response_rx: Mutex<MPMCReceiver<ConsoleResponse>>,
    exit_flag: AtomicBool,
    input_tag: AtomicU32,
}

impl ConsoleContext {
    pub fn new() -> Self {
        let send_bus = Mutex::new(Bus::new(10));
        let (response_tx, response_rx) = mpmc_queue(10);

        Self {
            commands: Default::default(),
            exit_flag: AtomicBool::new(false),
            send_bus,
            response_tx: Mutex::new(response_tx),
            response_rx: Mutex::new(response_rx),
            input_tag: AtomicU32::new(0),
        }
    }

    pub fn need_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub fn set_exit(&self) {
        self.exit_flag.store(true, Ordering::Relaxed)
    }

    pub fn get_send_rx(&self) -> BusReader<Vec<u8>> {
        self.send_bus.lock().unwrap().add_rx()
    }

    pub fn clone_reponse_tx(&self) -> MPMCSender<ConsoleResponse> {
        self.response_tx.lock().unwrap().clone()
    }

    pub fn append_command(&self, command: ProgramCommand) {
        self.commands.write().unwrap().push(command);
    }

    pub fn export_command(&self, from: usize) -> ProgramCommandArray {
        let mut arr = ProgramCommandArray::new();

        arr.set_commands(RepeatedField::from_slice(
            &self.commands.read().unwrap()[from..],
        ));

        arr
    }

    pub fn get_command_count(&self) -> usize {
        self.commands.read().unwrap().len()
    }

    pub fn get_next_input_tag(&self) -> u32 {
        self.input_tag.fetch_add(1, Ordering::Relaxed)
    }

    pub fn wait_console<F: FnMut(&mut ConsoleResponse) -> bool>(
        &self,
        mut req: ProgramRequest,
        mut pred: F,
    ) {
        {
            req.set_tag(self.get_next_input_tag());
            let mut msg = ProgramMessage::new();
            msg.set_REQ(req);

            let dat = Message::write_to_bytes(&msg).expect("serialize");

            self.send_bus.lock().unwrap().broadcast(dat);
        }

        loop {
            match self.response_rx.lock().unwrap().try_recv() {
                Ok(mut res) => {
                    if pred(&mut res) {
                        break;
                    }
                }

                Err(TryRecvError::Disconnected) => {
                    panic!("queue disconnected");
                }

                Err(TryRecvError::Empty) => {}
            };

            //TODO: implement timeout

            thread::sleep(Duration::from_millis(100));
        }
    }
}
