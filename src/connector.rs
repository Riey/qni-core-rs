use std::sync::Arc;
use bus::BusReader;

use crate::console::ConsoleContext;
use crate::protos::qni_api::*;

pub struct ConnectorContext {
    console_ctx: Arc<ConsoleContext>,
    send_rx: BusReader<ProgramMessage>,
}

impl ConnectorContext {
    pub fn new(console_ctx: Arc<ConsoleContext>) -> Self {
        Self {
            send_rx: console_ctx.get_send_rx(),
            console_ctx,
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
            self.console_ctx
                .on_recv_response(msg.take_RES())
                .map(|tag| {
                    let mut msg = ProgramMessage::new();
                    msg.set_ACCEPT_RES(tag);
                    msg
                })
        } else {
            None
        }
    }

    pub fn try_get_msg(&mut self) -> Option<ProgramMessage> {
        self.send_rx.try_recv().ok()
    }
}
