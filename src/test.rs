use crate::hub::*;
use crate::c_api::*;

unsafe fn qni_print_line_rust(ctx: ProgramEntryCtxArg, text: &str) {
    qni_print_line(ctx, text.as_ptr(), text.len());
}

fn test_entry(ctx: ProgramEntryCtxArg) {

    unsafe {
        qni_print_line_rust(ctx, "Hello, world!");
        qni_print_line_rust(ctx, "Hello, world!");
    }
}

use std::thread;
use std::time::Duration;

#[test]
fn api_test() {
    unsafe {
        let hub = { qni_hub_new(test_entry) };

        let ctx = (*hub).start_new_program();


        loop {
            if ctx.lock().unwrap().need_exit() {
                break;
            }

            thread::sleep(Duration::from_millis(20));
        }

        let ctx = ctx.lock().unwrap();

        assert_eq!(2, ctx.get_command_count());

        qni_hub_delete(hub);
    }
}
