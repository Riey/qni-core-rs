use crate::console::{ConsoleContext, WaitError};
use crate::protos::qni_api::*;

use std::mem;
use std::slice;
use std::sync::Arc;

use std::str;

/// ConsoleContext handle for ffi
pub type ConsoleArcCtx = *mut Arc<ConsoleContext>;

#[no_mangle]
pub unsafe extern "C" fn qni_console_new() -> ConsoleArcCtx {
    Box::into_raw(Box::new(Arc::new(ConsoleContext::new())))
}

#[no_mangle]
pub unsafe extern "C" fn qni_console_delete(ctx: ConsoleArcCtx) {
    let _ = Box::from_raw(ctx);
}

#[no_mangle]
pub unsafe extern "C" fn qni_console_exit(ctx: ConsoleArcCtx) {
    (*ctx).set_exit();
}

#[no_mangle]
pub unsafe extern "C" fn qni_console_need_exit(ctx: ConsoleArcCtx) -> i32 {
    if (*ctx).need_exit() {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_print(ctx: ConsoleArcCtx, text: *const u8, len: usize) -> i32 {
    if Arc::strong_count(&*ctx) <= 1 {
        -1
    } else {
        let mut command = ProgramCommand::new();
        let text = str::from_utf8_unchecked(slice::from_raw_parts(text, len));
        command.mut_PRINT().set_PRINT(text.into());

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_print_line(ctx: ConsoleArcCtx, text: *const u8, len: usize) {
    let mut command = ProgramCommand::new();
    let text = str::from_utf8_unchecked(slice::from_raw_parts(text, len));
    command.mut_PRINT().set_PRINT_LINE(text.into());

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_draw_line(ctx: ConsoleArcCtx) {
    let mut command = ProgramCommand::new();
    command.mut_PRINT().mut_DRAW_LINE();
    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_new_line(ctx: ConsoleArcCtx) {
    let mut command = ProgramCommand::new();
    command.mut_PRINT().mut_NEW_LINE();

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_delete_line(ctx: ConsoleArcCtx, count: u32) {
    let mut command = ProgramCommand::new();
    command.mut_PRINT().set_DELETE_LINE(count);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_font(
    ctx: ConsoleArcCtx,
    font_family: *const u8,
    font_family_len: usize,
    font_size: f32,
    font_style: u32,
) {
    let mut command = ProgramCommand::new();

    let mut font = Font::new();

    font.set_font_family(
        str::from_utf8_unchecked(slice::from_raw_parts(font_family, font_family_len)).into(),
    );
    font.set_font_size(font_size);
    font.set_font_style(font_style);

    command.mut_UPDATE_SETTING().set_FONT(font);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_text_align(ctx: ConsoleArcCtx, text_align: u32) {
    let mut command = ProgramCommand::new();
    command
        .mut_UPDATE_SETTING()
        .set_TEXT_ALIGN(mem::transmute(text_align as u8));

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_text_color(ctx: ConsoleArcCtx, color: u32) {
    let mut command = ProgramCommand::new();
    command.mut_UPDATE_SETTING().set_TEXT_COLOR(color);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_back_color(ctx: ConsoleArcCtx, color: u32) {
    let mut command = ProgramCommand::new();
    command.mut_UPDATE_SETTING().set_BACK_COLOR(color);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_highlight_color(ctx: ConsoleArcCtx, color: u32) {
    let mut command = ProgramCommand::new();
    command.mut_UPDATE_SETTING().set_HIGHLIGHT_COLOR(color);

    (*ctx).append_command(command);
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum QniWaitResult {
    Ok = 0,
    Exited = 1,
    Timeout = 2,
    OutDated = 3,
    Internal = -1,
}

#[no_mangle]
pub unsafe extern "C" fn qni_wait(
    ctx: ConsoleArcCtx,
    req: *mut ProgramRequest,
    out: *mut *mut ConsoleResponse,
) -> QniWaitResult {
    let req = Box::from_raw(req);

    match (*ctx).wait_console(*req) {
        Ok(res) => {
            *out = Box::into_raw(res);
            QniWaitResult::Ok
        },
        Err(WaitError::Exited) => QniWaitResult::Exited,
        Err(WaitError::Timeout) => QniWaitResult::Timeout,
        Err(WaitError::OutDated) => QniWaitResult::OutDated,
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_buf_delete(buf: *mut u8, len: usize, cap: usize) {
    let _ = Vec::from_raw_parts(buf, len, cap);
}

macro_rules! make_wait_fn {
    ($name:ident($ctx:ident: ConsoleArcCtx $($(,)? $arg_name:ident: $arg_ty:ty)*), $req_block:block, |$res:ident| $ok_block:block) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name($ctx: ConsoleArcCtx, $($arg_name: $arg_ty,)+) -> QniWaitResult {
            match (*$ctx).wait_console($req_block) {
                Ok(mut $res) => {
                    $ok_block
                    QniWaitResult::Ok
                }
                Err(WaitError::Exited) => QniWaitResult::Exited,
                Err(WaitError::Timeout) => QniWaitResult::Timeout,
                Err(WaitError::OutDated) => QniWaitResult::OutDated,
            }
        }
    };
}

make_wait_fn!(qni_wait_str(ctx: ConsoleArcCtx, buf: *mut *mut u8, buf_len: *mut usize, buf_cap: *mut usize), {
    let mut req = ProgramRequest::new();
    req.mut_INPUT().mut_STR();
    req
}, |res| {
    let mut text = res.take_OK_INPUT().take_STR().into_bytes();
    *buf = text.as_mut_ptr();
    *buf_len = text.len();
    *buf_cap = text.capacity();
});

make_wait_fn!(qni_wait_int(ctx: ConsoleArcCtx, num: *mut i32), {
    let mut req = ProgramRequest::new();
    req.mut_INPUT().mut_INT();
    req
}, |res| {
    *num = res.take_OK_INPUT().get_INT();
});
