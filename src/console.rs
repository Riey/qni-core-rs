use atomic_option::AtomicOption;
use chrono::prelude::*;
use protobuf::well_known_types::Timestamp;
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

/// Present ConsoleContext
pub struct ConsoleContext {
    commands: RwLock<Vec<ProgramCommand>>,
    exit_flag: AtomicBool,
    request_tag: AtomicUsize,
    request: RwLock<Option<ProgramRequest>>,
    response: AtomicOption<ConsoleResponse>,
}

impl ConsoleContext {

    /// Create new ConsoleContext
    pub fn new() -> Self {
        Self {
            commands: Default::default(),
            exit_flag: AtomicBool::new(false),
            request_tag: AtomicUsize::new(0),
            response: AtomicOption::empty(),
            request: RwLock::new(None),
        }
    }

    /// Console need exit
    pub fn need_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    /// Set console exit flag
    pub fn set_exit(&self) {
        self.exit_flag.store(true, Ordering::Relaxed)
    }

    /// Append console command
    pub fn append_command(&self, command: ProgramCommand) {
        self.commands.write().unwrap().push(command);
    }

    /// Export command to Vec
    pub fn export_command(&self, from: usize) -> Vec<ProgramCommand> {
        Vec::from(&self.commands.read().unwrap()[from..])
    }

    /// Get current command count
    #[inline]
    pub fn get_command_count(&self) -> usize {
        self.commands.read().unwrap().len()
    }

    /// Get next input tag
    #[inline]
    fn get_next_input_tag(&self) -> usize {
        self.request_tag.fetch_add(1, Ordering::Relaxed)
    }

    /// Get current input tag
    #[inline]
    pub fn get_cur_input_tag(&self) -> usize {
        self.request_tag.load(Ordering::Relaxed)
    }

    /// Receive ConsoleResponse message
    pub fn on_recv_response(&self, res: ConsoleResponse) {
        if !self.is_outdated_tag(res.get_tag() as usize) {
            self.response.swap(Box::new(res), Ordering::Release);
        }
    }

    /// Try get current ProgramRequest
    pub fn try_get_req(&self) -> Option<ProgramRequest> {
        self.request.read().unwrap().as_ref().map(Clone::clone)
    }

    /// Check if tag is outdated
    pub fn is_outdated_tag(&self, tag: usize) -> bool {
        tag + 1 < self.get_cur_input_tag()
    }

    /// Wait ConsoleResponse
    ///
    /// # Errors
    ///
    /// If Console exited, tag is outdated, or request is expired, then error is returned
    pub fn wait_console(&self, mut req: ProgramRequest) -> Result<Box<ConsoleResponse>, WaitError> {
        let tag = self.get_next_input_tag();

        let expire = if req.get_INPUT().has_expire() {
            let expire: &Timestamp = req.get_INPUT().get_expire();
            Some(Utc.timestamp(expire.seconds, expire.nanos as u32))
        } else {
            None
        };

        req.set_tag(tag as u32);

        *self.request.write().unwrap() = Some(req);

        let ret = loop {
            if self.need_exit() {
                break Err(WaitError::Exited);
            }

            let response = self.response.take(Ordering::Acquire);

            if let Some(response) = response {
                break Ok(response);
            }

            if let Some(expire) = expire {
                if Utc::now() >= expire {
                    break Err(WaitError::Timeout);
                }
            }

            if self.is_outdated_tag(tag) {
                break Err(WaitError::Timeout);
            }

            thread::sleep(Duration::from_millis(100));
        };

        ret
    }
}
