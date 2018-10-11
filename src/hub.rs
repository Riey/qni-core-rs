use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::console::ConsoleContext;

pub type ProgramEntryCtxArg = *mut Arc<Mutex<ConsoleContext>>;
pub type ProgramEntryFuncPtr = fn(ProgramEntryCtxArg) -> ();

#[derive(Copy, Clone)]
pub struct ProgramEntryCallback(pub ProgramEntryFuncPtr);

unsafe impl Send for ProgramEntryCallback {}
unsafe impl Sync for ProgramEntryCallback {}

pub struct Hub {
    entry: ProgramEntryCallback,
    shared_ctxs: BTreeMap<String, Arc<Mutex<ConsoleContext>>>,
    exit_flag: bool,
}

impl Hub {
    pub fn new(entry: ProgramEntryCallback) -> Self {
        Self {
            entry,
            shared_ctxs: Default::default(),
            exit_flag: false,
        }
    }

    pub fn on_console_ctx_removed(ctx: &Arc<Mutex<ConsoleContext>>) {
        if Arc::strong_count(ctx) <= 2 {
            ctx.lock().unwrap().set_exit();
        }
    }

    pub fn need_exit(&self) -> bool {
        self.exit_flag
    }

    pub fn set_exit(&mut self) {
        self.exit_flag = true;
    }

    pub fn start_new_program(&self) -> Arc<Mutex<ConsoleContext>> {
        let ctx = Arc::new(Mutex::new(ConsoleContext::new()));

        {
            let entry = self.entry;
            let ctx = ctx.clone();

            thread::spawn(move || {
                let ctx_box = Box::new(ctx.clone());
                entry.0(Box::into_raw(ctx_box));

                ctx.lock().unwrap().set_exit();
            });
        }

        ctx
    }

    pub fn insert_ctx(
        &mut self,
        key: String,
        ctx: &Arc<Mutex<ConsoleContext>>,
        overwrite: bool,
    ) -> bool {
        match overwrite {
            true => {
                let prev = self.shared_ctxs.insert(key, ctx.clone());

                if let Some(prev) = prev {
                    Hub::on_console_ctx_removed(&prev);
                }
            }

            false => {
                if self.shared_ctxs.contains_key(&key) {
                    return false;
                }

                self.shared_ctxs.insert(key, ctx.clone());
            }
        };

        true
    }

    pub fn erase_ctx(&mut self, key: &str) -> bool {
        match self.shared_ctxs.remove(key) {
            Some(prev) => {
                Hub::on_console_ctx_removed(&prev);
                true
            }
            None => false,
        }
    }

    pub fn get_ctx(&self, key: &str) -> Option<Arc<Mutex<ConsoleContext>>> {
        match self.shared_ctxs.get(key) {
            Some(ctx) => Some(ctx.clone()),
            None => None,
        }
    }
}
