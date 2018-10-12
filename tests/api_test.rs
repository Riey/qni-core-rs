use qni_core_rs::c_api::*;
use qni_core_rs::prelude::qni_api::*;
use qni_core_rs::prelude::*;

unsafe fn qni_print_line_rust(ctx: ProgramEntryCtxArg, text: &str) {
    qni_print_line(ctx, text.as_ptr(), text.len());
}

extern "C" fn test_simple_entry(ctx: ProgramEntryCtxArg) {
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

        let ctx = (*hub).start_new_program();

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

extern "C" fn test_wait_entry(ctx: ProgramEntryCtxArg) {
    unsafe {
        let mut ret = 0;
        assert_eq!(0, qni_wait_int(ctx, &mut ret));
        assert_eq!(100, ret);
    }
}

use std::sync::mpsc::TryRecvError;

#[test]
fn api_wait_test() {
    unsafe {
        let hub = { qni_hub_new(test_wait_entry) };

        let ctx = (*hub).start_new_program();

        let mut msg = ProgramMessage::new();
        let input_req = msg.mut_REQ();

        input_req.mut_INPUT().mut_INT();
        input_req.set_tag(0);

        let mut connector_ctx = ConnectorContext::new(hub.read(), ctx.clone());

        loop {
            match connector_ctx.try_recv_send_messge() {
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

        assert_eq!(
            connector_ctx.recv_message(&protobuf::Message::write_to_bytes(&msg).unwrap()),
            None
        );

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
