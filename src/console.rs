use atomic_option::AtomicOption;
use chrono::prelude::*;
use protobuf::well_known_types::Timestamp;
use protobuf::RepeatedField;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

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
    request: RwLock<Option<ProgramRequest>>,
    response: AtomicOption<ConsoleResponse>,
}

impl ConsoleContext {
    pub fn new() -> Self {
        Self {
            commands: Default::default(),
            exit_flag: AtomicBool::new(false),
            request_tag: AtomicUsize::new(0),
            response: AtomicOption::empty(),
            request: RwLock::new(None),
        }
    }

    pub fn need_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub fn set_exit(&self) {
        self.exit_flag.store(true, Ordering::Relaxed)
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

    #[inline]
    pub fn get_command_count(&self) -> usize {
        self.commands.read().unwrap().len()
    }

    #[inline]
    pub fn get_next_input_tag(&self) -> usize {
        self.request_tag.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn get_cur_input_tag(&self) -> usize {
        self.request_tag.load(Ordering::Relaxed)
    }

    pub fn on_recv_response(&self, res: ConsoleResponse) {
        //outdated
        if res.tag + 1 < self.get_cur_input_tag() as u32 {
        } else {
            self.response.swap(Box::new(res), Ordering::Release);
        }
    }

    pub fn try_get_req(&self) -> Option<ProgramRequest> {
        self.request.read().unwrap().as_ref().map(Clone::clone)
    }

    pub fn wait_console<F: FnMut(&mut ConsoleResponse) -> bool>(
        &self,
        mut req: ProgramRequest,
        mut pred: F,
    ) -> Result<(), WaitError> {
        let tag = self.get_next_input_tag();

        let expire = if req.get_INPUT().has_expire() {
            let expire: &Timestamp = req.get_INPUT().get_expire();
            Some(Utc.timestamp(expire.seconds, expire.nanos as u32))
        } else {
            None
        };

        req.set_tag(tag as _);

        *self.request.write().unwrap() = Some(req);

        loop {
            if self.need_exit() {
                *self.request.write().unwrap() = None;
                return Err(WaitError::Exited);
            }

            let response = self.response.take(Ordering::Acquire);

            if let Some(mut response) = response {
                if pred(&mut response) {
                    *self.request.write().unwrap() = None;
                    break;
                }
            }

            if let Some(expire) = expire {
                if Utc::now() >= expire {
                    *self.request.write().unwrap() = None;
                    return Err(WaitError::Timeout);
                }
            }

            thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }
}
