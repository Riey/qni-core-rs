use multiqueue::{broadcast_queue, BroadcastReceiver, BroadcastSender};
use protobuf::{Message, RepeatedField};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr, Ordering};
use std::sync::mpsc::TrySendError;
use std::sync::{RwLock, Mutex};
use std::thread;
use std::time::Duration;
use chrono::prelude::*;

use crate::protos::qni_api::*;

#[derive(Debug)]
pub enum WaitError {
    Timeout,
    Exited,
}

pub struct ConsoleContext {
    commands: RwLock<Vec<ProgramCommand>>,
    exit_flag: AtomicBool,
    request_tag: AtomicUsize,
    send_tx: Mutex<BroadcastSender<Vec<u8>>>,
    send_rx: Mutex<BroadcastReceiver<Vec<u8>>>,
    response: AtomicPtr<ConsoleResponse>,
}

impl Drop for ConsoleContext {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.response.load(Ordering::Relaxed));
        }
    }
}

impl ConsoleContext {
    pub fn new() -> Self {
        let (send_tx, send_rx) = broadcast_queue(10);

        Self {
            commands: Default::default(),
            send_tx: Mutex::new(send_tx),
            send_rx: Mutex::new(send_rx),
            exit_flag: AtomicBool::new(false),
            request_tag: AtomicUsize::new(0),
            response: AtomicPtr::new(ptr::null_mut()),
        }
    }

    pub fn need_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub fn set_exit(&self) {
        self.exit_flag.store(true, Ordering::Relaxed)
    }

    pub fn get_send_rx(&self) -> BroadcastReceiver<Vec<u8>> {
        self.send_rx.lock().unwrap().clone()
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
        self.request_tag.fetch_add(1, Ordering::Relaxed) as u32
    }

    #[inline]
    pub fn get_cur_input_tag(&self) -> u32 {
        self.request_tag.load(Ordering::Relaxed) as u32
    }

    pub fn on_recv_response(&self, res: ConsoleResponse) -> Option<u32> {
        //outdated
        if res.tag + 1 < self.get_cur_input_tag() {
            Some(res.tag)
        } else {
            unsafe {
                let res = Box::new(res);

                let ptr = self.response.swap(Box::into_raw(res), Ordering::Relaxed);

                if ptr != ptr::null_mut() {
                    let _ = Box::from_raw(ptr);
                }

                None
            }
        }
    }

    pub fn wait_console<F: FnMut(&mut ConsoleResponse) -> bool, FE: Fn() -> bool>(
        &self,
        mut req: ProgramRequest,
        pred_exit: FE,
        mut pred: F,
        expire: Option<DateTime<Utc>>,
    ) -> Result<(), WaitError> {
        let tag = self.get_next_input_tag();
        let mut msg = ProgramMessage::new();

        {
            req.set_tag(tag);
            msg.set_REQ(req);

            let mut dat = Message::write_to_bytes(&msg).expect("serialize");

            loop {
                match self.send_tx.lock().unwrap().try_send(dat) {
                    Ok(_) => break,
                    Err(TrySendError::Disconnected(prev_dat))
                    | Err(TrySendError::Full(prev_dat)) => {
                        dat = prev_dat;
                    }
                }

                thread::sleep(Duration::from_millis(50));
            }

            msg.clear_REQ();
        }

        loop {
            if pred_exit() {
                self.set_exit();
            }

            if self.need_exit() {
                return Err(WaitError::Exited);
            }

            let response = self.response.swap(ptr::null_mut(), Ordering::Relaxed);

            if response != ptr::null_mut() {
                unsafe {
                    let result = pred(&mut *response);

                    let _ = Box::from_raw(response);

                    if result {
                        break;
                    }
                }
            }

            if let Some(expire) = expire {
                if Utc::now() >= expire {
                    return Err(WaitError::Timeout);
                }
            }

            thread::sleep(Duration::from_millis(100));
        }

        msg.set_ACCEPT_RES(tag);

        let mut dat = Message::write_to_bytes(&msg).expect("serialize");

        loop {
            match self.send_tx.lock().unwrap().try_send(dat) {
                Ok(_) => break,
                Err(TrySendError::Disconnected(prev_dat)) | Err(TrySendError::Full(prev_dat)) => {
                    dat = prev_dat;
                }
            }

            thread::sleep(Duration::from_millis(50));
        }

        Ok(())
    }
}
