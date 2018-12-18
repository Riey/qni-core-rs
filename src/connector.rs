use std::sync::Arc;

use crate::console::ConsoleContext;
use crate::protos::qni_api::*;
use log::{debug, error};
use multiqueue::BroadcastReceiver;
use protobuf::Message;

pub struct ConnectorContext {
    console_ctx: Arc<ConsoleContext>,
    send_rx: BroadcastReceiver<Vec<u8>>,
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

    fn process_request(&mut self, req: ConsoleRequest) -> Option<Vec<u8>> {
        if let Some(req_data) = req.data {
            let mut msg = ProgramMessage::new();

            match req_data {
                ConsoleRequest_oneof_data::GET_STATE(from) => {
                    let ctx = &self.console_ctx;

                    let from = from as usize;

                    if ctx.need_exit() && from >= ctx.get_command_count() {
                        let err = msg.mut_RES().mut_ERR();
                        err.set_reason("program exited".into());
                        err.set_req_type("GET_STATE".into());
                    } else {
                        msg.mut_RES().set_OK_GET_STATE(ctx.export_command(from));
                    }
                }
            }

            debug!("response: {:#?}", msg);

            Some(Message::write_to_bytes(&msg).expect("serialize"))
        } else {
            None
        }
    }

    pub fn on_recv_message(&mut self, msg: &[u8]) -> Option<Vec<u8>> {
        match protobuf::parse_from_bytes::<ConsoleMessage>(msg) {
            Ok(mut msg) => {
                debug!("received: {:#?}", msg);

                if msg.has_REQ() {
                    self.process_request(msg.take_REQ())
                } else if msg.has_RES() {
                    self.console_ctx
                        .on_recv_response(msg.take_RES())
                        .map(|tag| {
                            let mut msg = ProgramMessage::new();
                            msg.set_ACCEPT_RES(tag);
                            Message::write_to_bytes(&msg).expect("serialize")
                        })
                } else {
                    None
                }
            }
            Err(err) => {
                error!("failed to read msg: {}", err);

                None
            }
        }
    }

    pub fn try_get_msg(&mut self) -> Option<Vec<u8>> {
        self.send_rx.try_recv().ok()
    }
}
