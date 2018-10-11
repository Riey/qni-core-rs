use std::sync::{mpsc::{TryRecvError, TrySendError}, Arc, Mutex};

use crate::console::ConsoleContext;
use crate::hub::Hub;
use crate::protos::qni_api::*;
use multiqueue::MPMCSender;
use protobuf::Message;
use std;
use bus::BusReader;

pub struct ConnectorContext {
    hub: Arc<Mutex<Hub>>,
    console_ctx: Arc<ConsoleContext>,
    send_rx: BusReader<Vec<u8>>,
    response_tx: MPMCSender<ConsoleResponse>,
}

impl ConnectorContext {
    pub fn new(hub: Arc<Mutex<Hub>>, console_ctx: Arc<ConsoleContext>) -> Self {
        Self {
            hub,
            send_rx: console_ctx.get_send_rx(),
            response_tx: console_ctx.clone_reponse_tx(),
            console_ctx,
        }
    }

    pub fn update_console_ctx(&mut self, console_ctx: Arc<ConsoleContext>) {
        self.send_rx = console_ctx.get_send_rx();
        self.response_tx = console_ctx.clone_reponse_tx();
        self.console_ctx = console_ctx;
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
                        err.set_reason("program exited".to_string());
                        err.set_req_type("GET_STATE".to_string());
                    } else {
                        msg.mut_RES().set_OK_GET_STATE(ctx.export_command(from));
                    }
                }
                ConsoleRequest_oneof_data::LOAD_STATE(name) => {
                    let ctx = self.hub.lock().unwrap().get_ctx(&name);

                    match ctx {
                        Some(ctx) => {
                            self.update_console_ctx(ctx);
                            msg.mut_RES().mut_OK_LOAD_STATE();
                        }
                        None => {
                            let err = msg.mut_RES().mut_ERR();
                            err.set_reason(format!("state [{}] not exist", name));
                            err.set_req_type("LOAD_STATE".to_string());
                        }
                    }
                }

                ConsoleRequest_oneof_data::SHARE_STATE(name) => {
                    match self.hub.lock().unwrap().insert_ctx(
                        name.clone(),
                        &self.console_ctx,
                        false,
                    ) {
                        true => {
                            msg.mut_RES().set_OK_SHARE_STATE(name);
                        }
                        false => {
                            let err = msg.mut_RES().mut_ERR();
                            err.set_reason(format!("state [{}] already exist", name));
                            err.set_req_type("SHARE_STATE".to_string());
                        }
                    }
                }

                ConsoleRequest_oneof_data::SHARE_STATE_OVERWRITE(name) => {
                    self.hub
                        .lock()
                        .unwrap()
                        .insert_ctx(name.clone(), &self.console_ctx, true);
                    msg.mut_RES().set_OK_SHARE_STATE(name);
                }

                ConsoleRequest_oneof_data::DELETE_STATE(name) => {
                    self.hub.lock().unwrap().erase_ctx(&name);
                }
            }

            Some(Message::write_to_bytes(&msg).expect("serialize"))
        } else {
            None
        }
    }

    pub fn try_recv_send_messge(&mut self) -> Result<Vec<u8>, TryRecvError> {
        self.send_rx.try_recv()
    }

    pub fn recv_message(&mut self, msg: &[u8]) -> Option<Vec<u8>> {
        match protobuf::parse_from_bytes::<ConsoleMessage>(msg) {
            Ok(mut msg) => {
                if msg.has_REQ() {
                    self.process_request(msg.take_REQ())
                } else if msg.has_RES() {
                    let mut res = msg.take_RES();
                    loop {
                        match self.response_tx.try_send(res) {
                            Ok(_) => break,
                            Err(TrySendError::Full(left_res)) => {
                                res = left_res;
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                return None;
                            }
                        }
                        if self.response_tx.try_send(msg.take_RES()).is_ok() {
                            break;
                        }

                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    None
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
