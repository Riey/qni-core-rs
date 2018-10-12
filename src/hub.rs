use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock, Weak,
};
use std::thread;

use crate::console::ConsoleContext;

pub type ProgramEntryCtxArg = *mut Weak<ConsoleContext>;
pub type ProgramEntryFuncPtr = extern "C" fn(ProgramEntryCtxArg) -> ();
pub type SharedHubPtr = *mut Arc<Hub>;

#[derive(Copy, Clone)]
pub struct ProgramEntryCallback(pub ProgramEntryFuncPtr);

unsafe impl Send for ProgramEntryCallback {}
unsafe impl Sync for ProgramEntryCallback {}

pub struct Hub {
    entry: ProgramEntryCallback,
    shared_ctxs: RwLock<BTreeMap<String, Arc<ConsoleContext>>>,
    exit_flag: AtomicBool,
}

impl Hub {
    pub fn new(entry: ProgramEntryCallback) -> Self {
        Self {
            entry,
            shared_ctxs: Default::default(),
            exit_flag: AtomicBool::new(false),
        }
    }

    pub fn need_exit(&self) -> bool {
        self.exit_flag.load(Ordering::Relaxed)
    }

    pub fn set_exit(&self) {
        self.exit_flag.store(true, Ordering::Relaxed);
    }

    pub fn start_new_program(&self) -> Arc<ConsoleContext> {
        let ctx = Arc::new(ConsoleContext::new());

        {
            let entry = self.entry;
            let ctx = Arc::downgrade(&ctx);

            thread::spawn(move || {
                let ctx_box = Box::new(ctx.clone());
                entry.0(Box::into_raw(ctx_box));

                if let Some(ctx) = ctx.upgrade() {
                    ctx.set_exit();
                }
            });
        }

        ctx
    }

    pub fn insert_ctx(&self, key: String, ctx: &Arc<ConsoleContext>, overwrite: bool) -> bool {
        match overwrite {
            true => {
                self.shared_ctxs.write().unwrap().insert(key, ctx.clone());
            }

            false => {
                let mut shared_ctxs = self.shared_ctxs.write().unwrap();

                if shared_ctxs.contains_key(&key) {
                    return false;
                }

                shared_ctxs.insert(key, ctx.clone());
            }
        };

        true
    }

    pub fn erase_ctx(&self, key: &str) -> bool {
        match self.shared_ctxs.write().unwrap().remove(key) {
            Some(_) => true,
            None => false,
        }
    }

    pub fn get_ctx(&self, key: &str) -> Option<Arc<ConsoleContext>> {
        match self.shared_ctxs.read().unwrap().get(key) {
            Some(ctx) => Some(ctx.clone()),
            None => None,
        }
    }
}
