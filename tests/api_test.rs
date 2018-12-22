use qni_core_rs::c_api::*;
use qni_core_rs::prelude::qni_api::*;
use qni_core_rs::prelude::*;
use std::mem;

static mut EXIT_FLAG: bool = false;
static mut EXIT_VALUE: QniWaitResult = QniWaitResult::Ok;

unsafe fn qni_wait_int(ctx: ConsoleArcCtx, ret: &mut i32) -> QniWaitResult {
    let mut req = ProgramRequest::new();
    req.mut_INPUT().mut_INT();
    let mut buf = protobuf::Message::write_to_bytes(&req).unwrap();
    let mut ret_vec = mem::uninitialized();

    let wait_ret = qni_wait(ctx, buf.as_mut_ptr(), buf.len(), &mut ret_vec);
    let res = protobuf::parse_from_bytes::<ConsoleResponse>(&ret_vec.into_vec()).unwrap();

    *ret = res.get_OK_INPUT().get_INT();

    wait_ret
}

extern "C" fn test_exit_entry(ctx: ConsoleArcCtx) {
    unsafe {
        let mut ret = 0;

        EXIT_FLAG = true;

        EXIT_VALUE = qni_wait_int(ctx, &mut ret);
        qni_console_exit(ctx);
    }
}

#[test]
fn api_exit_test() {
    unsafe {
        let ctx = Arc::new(ConsoleContext::new());

        let handle = {
            let mut ctx = ctx.clone();
            thread::spawn(move || {
                test_exit_entry(&mut ctx as _);
            })
        };

        loop {
            if EXIT_FLAG {
                break;
            }

            thread::sleep(Duration::from_millis(20));
        }

        ctx.set_exit();

        loop {
            if EXIT_VALUE != QniWaitResult::Ok {
                break;
            }

            thread::sleep(Duration::from_millis(20));
        }

        handle.join().unwrap();

        assert_eq!(QniWaitResult::Exited, EXIT_VALUE);
    }
}

unsafe fn qni_print_line_rust(ctx: ConsoleArcCtx, text: &str) {
    qni_print_line(ctx, text.as_ptr(), text.len());
}

extern "C" fn test_simple_entry(ctx: ConsoleArcCtx) {
    unsafe {
        qni_print_line_rust(ctx, "Hello, world!");
        qni_print_line_rust(ctx, "Hello, world!");
        qni_console_exit(ctx);
    }
}

use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn api_simple_test() {
    let mut ctx = Arc::new(ConsoleContext::new());
    test_simple_entry(&mut ctx as _);

    loop {
        if ctx.need_exit() {
            break;
        }

        thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(2, ctx.get_command_count());
}

extern "C" fn test_wait_entry(ctx: ConsoleArcCtx) {
    unsafe {
        let mut ret = 0;
        if qni_wait_int(ctx, &mut ret) == QniWaitResult::Ok {
            assert_eq!(100, ret);
        }
        qni_console_exit(ctx);
    }
}

#[test]
fn api_delete_test() {
    unsafe {
        let ctx = qni_console_new();
        qni_console_delete(ctx);

        let text = vec![1, 2, 3];
        qni_vec_delete(&mut QniVec::from_vec(text));
    }
}

#[test]
fn api_wait_test() {
    let ctx = Arc::new(ConsoleContext::new());
    {
        let mut ctx = ctx.clone();
        thread::spawn(move || {
            test_wait_entry(&mut ctx as *mut _);
        })
    };

    let mut msg = ProgramMessage::new();
    let input_req = msg.mut_REQ();

    input_req.mut_INPUT().mut_INT();
    input_req.set_tag(0);

    let connector_ctx = ConnectorContext::new(ctx.clone());

    loop {
        match connector_ctx.try_get_msg() {
            Some(send_msg) => {
                assert_eq!(msg, send_msg);
                break;
            }
            None => {
                thread::sleep(Duration::from_millis(50));
            }
        }
    }

    let mut msg = ConsoleMessage::new();
    let input_res = msg.mut_RES();

    input_res.mut_OK_INPUT().set_INT(100);

    assert_eq!(
        connector_ctx.on_recv_message(msg),
        None
    );

    loop {
        if ctx.need_exit() {
            break;
        }

        thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(0, ctx.get_command_count());

    let mut msg = ProgramMessage::new();
    msg.set_ACCEPT_RES(0);

    assert_eq!(
        msg,
        connector_ctx.try_get_msg().unwrap()
    );
}
