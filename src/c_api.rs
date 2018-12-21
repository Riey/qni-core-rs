use crate::console::{ConsoleContext, WaitError};
use crate::protos::qni_api::*;

use std::mem;
use std::slice;
use std::sync::Arc;

use std::str;

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
    Exited = -1,
    Timeout = 1,
}

impl From<Result<(), WaitError>> for QniWaitResult {
    fn from(ret: Result<(), WaitError>) -> Self {
        match ret {
            Ok(_) => QniWaitResult::Ok,
            Err(WaitError::Exited) => QniWaitResult::Exited,
            Err(WaitError::Timeout) => QniWaitResult::Timeout,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_wait_int(ctx: ConsoleArcCtx, ret: *mut i32) -> QniWaitResult {
    let mut req = ProgramRequest::new();
    req.mut_INPUT().mut_INT();

    (*ctx)
        .wait_console(req, |res| {
            if !res.has_OK_INPUT() {
                return false;
            }

            match res.take_OK_INPUT().data {
                Some(InputResponse_oneof_data::INT(num)) => {
                    *ret = num;
                    true
                }
                _ => false,
            }
        })
        .into()
}

#[no_mangle]
pub unsafe extern "C" fn qni_str_delete(ptr: *mut u8, len: usize, cap: usize) {
    let _ = String::from_raw_parts(ptr, len, cap);
}

#[no_mangle]
pub unsafe extern "C" fn qni_wait_str(
    ctx: ConsoleArcCtx,
    ret: *mut *mut u8,
    ret_len: *mut usize,
    ret_cap: *mut usize,
) -> QniWaitResult {
    let mut req = ProgramRequest::new();
    req.mut_INPUT().mut_INT();

    (*ctx)
        .wait_console(req, |res| {
            if !res.has_OK_INPUT() {
                return false;
            }

            match res.take_OK_INPUT().data {
                Some(InputResponse_oneof_data::STR(mut text)) => {
                    *ret = text.as_bytes_mut().as_mut_ptr();
                    *ret_len = text.len();
                    *ret_cap = text.capacity();
                    mem::forget(text);
                    true
                }
                _ => false,
            }
        })
        .into()
}
