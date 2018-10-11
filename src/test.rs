use crate::c_api::*;
use crate::prelude::qni_api::*;
use crate::prelude::*;

unsafe fn qni_print_line_rust(ctx: ProgramEntryCtxArg, text: &str) {
    qni_print_line(ctx, text.as_ptr(), text.len());
}

fn test_simple_entry(ctx: ProgramEntryCtxArg) {
    unsafe {
        qni_print_line_rust(ctx, "Hello, world!");
        qni_print_line_rust(ctx, "Hello, world!");
    }
}

use std::thread;
use std::time::Duration;

#[test]
fn api_simple_test() {
    unsafe {
        let hub = { qni_hub_new(test_simple_entry) };

        let ctx = (*hub).lock().unwrap().start_new_program();

        loop {
            if ctx.need_exit() {
                break;
            }

            thread::sleep(Duration::from_millis(20));
        }

        assert_eq!(2, ctx.get_command_count());

        qni_hub_delete(hub);
    }
}

fn test_wait_entry(ctx: ProgramEntryCtxArg) {
    unsafe {
        assert_eq!(100, qni_wait_int(ctx));
    }
}

use std::sync::mpsc::TryRecvError;

#[test]
fn api_wait_test() {
    unsafe {
        let hub = { qni_hub_new(test_wait_entry) };

        let ctx = (*hub).lock().unwrap().start_new_program();

        let mut msg = ProgramMessage::new();
        let input_req = msg.mut_REQ();

        input_req.mut_INPUT().mut_INT();
        input_req.set_tag(0);

        let mut send_rx = ctx.get_send_rx();

        loop {
            match send_rx.try_recv() {
                Ok(send_msg) => {
                    assert_eq!(msg, protobuf::parse_from_bytes(&send_msg).unwrap());
                    break;
                }
                Err(TryRecvError::Disconnected) => panic!("disconnected"),
                Err(TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        let mut msg = ConsoleMessage::new();
        let input_res = msg.mut_RES();

        input_res.mut_OK_INPUT().set_INT(100);

        ctx.clone_reponse_tx().try_send(input_res.clone()).unwrap();

        loop {
            if ctx.need_exit() {
                break;
            }

            thread::sleep(Duration::from_millis(20));
        }

        assert_eq!(0, ctx.get_command_count());

        qni_hub_delete(hub);
    }
}
