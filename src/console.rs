use crate::protos::qni_api::*;
use multiqueue::*;
use protobuf::{Message, RepeatedField};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TrySendError};
use std::thread;
use std::time::Duration;

pub struct ConsoleContext {
    commands: Vec<ProgramCommand>,
    send_tx: MPMCSender<Vec<u8>>,
    send_rx: MPMCReceiver<Vec<u8>>,
    response_tx: Sender<ConsoleResponse>,
    response_rx: Receiver<ConsoleResponse>,
    exit_flag: bool,
}

impl ConsoleContext {
    pub fn new() -> Self {
        let (send_tx, send_rx) = mpmc_queue(10);
        let (response_tx, response_rx) = mpsc::channel();

        Self {
            commands: Default::default(),
            exit_flag: false,
            send_tx,
            send_rx,
            response_tx,
            response_rx,
        }
    }

    pub fn need_exit(&self) -> bool {
        self.exit_flag
    }

    pub fn set_exit(&mut self) {
        self.exit_flag = true;
    }

    pub fn clone_send_rx(&self) -> MPMCReceiver<Vec<u8>> {
        self.send_rx.clone()
    }

    pub fn clone_reponse_tx(&self) -> Sender<ConsoleResponse> {
        self.response_tx.clone()
    }

    pub fn append_command(&mut self, command: ProgramCommand) {
        self.commands.push(command);
    }

    pub fn export_command(&self, from: usize) -> ProgramCommandArray {
        let mut arr = ProgramCommandArray::new();

        arr.set_commands(RepeatedField::from_slice(&self.commands[from..]));

        arr
    }

    pub fn get_command_count(&self) -> usize {
        self.commands.len()
    }

    pub fn wait_console<F: FnMut(&mut ConsoleResponse) -> bool>(
        &self,
        req: ProgramRequest,
        mut pred: F,
    ) {
        {
            let mut msg = ProgramMessage::new();
            msg.set_REQ(req);

            let mut dat = Message::write_to_bytes(&msg).expect("serialize");

            loop {
                match self.send_tx.try_send(dat) {
                    Ok(()) => break,
                    Err(TrySendError::Disconnected(_)) => {
                        panic!("queue disconnected");
                    }
                    Err(TrySendError::Full(prev_dat)) => {
                        dat = prev_dat;
                    }
                }
                thread::sleep(Duration::from_millis(200));
            }
        }

        loop {
            let res = self.response_rx.recv_timeout(Duration::from_millis(200));

            match res {
                Ok(mut res) => {
                    if pred(&mut res) {
                        break;
                    }
                }

                Err(RecvTimeoutError::Disconnected) => {
                    panic!("queue disconnected");
                }

                Err(RecvTimeoutError::Timeout) => {}
            };

            //TODO: implement timeout

            thread::sleep(Duration::from_millis(100));
        }
    }
}
