use libc::{free, malloc};
use multiqueue::{broadcast_queue, BroadcastReceiver, BroadcastSender};
use protobuf::{Message, RepeatedField};
use std::mem::size_of;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
#[cfg(NIGHTLY)]
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::TrySendError;
use std::sync::RwLock;
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
    #[cfg(NIGHTLY)]
    request_tag: AtomicU32,
    #[cfg(not(NIGHTLY))]
    request_tag: RwLock<u32>,
    send_tx: BroadcastSender<Vec<u8>>,
    send_rx: BroadcastReceiver<Vec<u8>>,
    response: AtomicPtr<ConsoleResponse>,
}

unsafe impl Sync for ConsoleContext {}

impl Drop for ConsoleContext {
    fn drop(&mut self) {
        unsafe {
            free(self.response.load(Ordering::Relaxed) as *mut _);
        }
    }
}

impl ConsoleContext {
    pub fn new() -> Self {
        let (send_tx, send_rx) = broadcast_queue(10);

        Self {
            commands: Default::default(),
            send_tx,
            send_rx,
            exit_flag: AtomicBool::new(false),
            #[cfg(NIGHTLY)]
            request_tag: AtomicU32::new(0),
            #[cfg(not(NIGHTLY))]
            request_tag: RwLock::new(0),
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
        self.send_rx.clone()
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
        #[cfg(NIGHTLY)]
        {
            self.request_tag.fetch_add(1, Ordering::Relaxed)
        }
        #[cfg(not(NIGHTLY))]
        {
            let mut tag = self.request_tag.write().unwrap();
            let temp = *tag;
            *tag += 1;
            temp
        }
    }

    #[inline]
    pub fn get_cur_input_tag(&self) -> u32 {
        #[cfg(NIGHTLY)]
        {
            self.request_tag.load(Ordering::Relaxed)
        }
        #[cfg(not(NIGHTLY))]
        {
            *self.request_tag.read().unwrap()
        }
    }

    pub fn on_recv_response(&self, res: ConsoleResponse) -> Option<u32> {
        //outdated
        if res.tag + 1 < self.get_cur_input_tag() {
            Some(res.tag)
        } else {
            unsafe {
                let mut ptr = malloc(size_of::<ConsoleResponse>()) as *mut ConsoleResponse;
                ptr.write(res);

                ptr = self.response.swap(ptr, Ordering::Relaxed);

                free(ptr as *mut _);

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
                match self.send_tx.try_send(dat) {
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

                    free(response as *mut _);

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
            match self.send_tx.try_send(dat) {
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
