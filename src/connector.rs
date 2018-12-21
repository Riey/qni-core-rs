use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::console::ConsoleContext;
use crate::protos::qni_api::*;

pub struct ConnectorContext {
    console_ctx: Arc<ConsoleContext>,
    last_req_tag: AtomicUsize,
    last_sended_req_tag: AtomicUsize,
}

impl ConnectorContext {
    pub fn new(console_ctx: Arc<ConsoleContext>) -> Self {
        Self {
            console_ctx,
            last_req_tag: AtomicUsize::new(0),
            last_sended_req_tag: AtomicUsize::new(0),
        }
    }

    pub fn need_exit(&self) -> bool {
        self.console_ctx.need_exit()
    }

    fn process_request(&self, req: ConsoleRequest) -> Option<ProgramResponse> {
        if let Some(req_data) = req.data {
            let mut res = ProgramResponse::new();

            match req_data {
                ConsoleRequest_oneof_data::GET_STATE(from) => {
                    let ctx = &self.console_ctx;

                    let from = from as usize;

                    if ctx.need_exit() && from >= ctx.get_command_count() {
                        let err = res.mut_ERR();
                        err.set_reason("program exited".into());
                        err.set_req_type("GET_STATE".into());
                    } else {
                        res.set_OK_GET_STATE(ctx.export_command(from));
                    }
                }
            }

            Some(res)
        } else {
            None
        }
    }

    pub fn on_recv_message(&self, mut msg: ConsoleMessage) -> Option<ProgramMessage> {
        if msg.has_REQ() {
            self.process_request(msg.take_REQ()).map(|res| {
                let mut msg = ProgramMessage::new();
                msg.set_RES(res);
                msg
            })
        } else if msg.has_RES() {
            self.console_ctx.on_recv_response(msg.take_RES());

            None
        } else {
            None
        }
    }

    pub fn try_get_msg(&self) -> Option<ProgramMessage> {
        if self.console_ctx.get_cur_input_tag() > self.last_req_tag.load(Ordering::Relaxed) {
            self.console_ctx.try_get_req().map(|req| {
                self.last_req_tag
                    .store(req.tag as usize + 1, Ordering::Relaxed);
                let mut msg = ProgramMessage::new();
                msg.set_REQ(req);
                msg
            })
        } else {
            let sended_tag = self.last_sended_req_tag.load(Ordering::Relaxed);
            let last_tag = self.last_req_tag.load(Ordering::Relaxed);

            if sended_tag < last_tag {
                let mut msg = ProgramMessage::new();
                msg.set_ACCEPT_RES(last_tag as u32 - 1);
                self.last_sended_req_tag.store(last_tag, Ordering::Relaxed);
                Some(msg)
            } else {
                None
            }
        }
    }
}
