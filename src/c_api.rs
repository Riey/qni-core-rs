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

/// Present Vec<u8> for ffi
#[repr(C)]
pub struct QniVec {
    ptr: *mut u8,
    len: usize,
    cap: usize,
}

impl QniVec {
    pub unsafe fn into_vec(self) -> Vec<u8> {
        Vec::from_raw_parts(self.ptr, self.len, self.cap)
    }

    pub fn from_vec(mut vec: Vec<u8>) -> Self {
        let ret = Self {
            ptr: vec.as_mut_ptr(),
            len: vec.len(),
            cap: vec.capacity(),
        };

        mem::forget(vec);

        ret
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum QniWaitResult {
    Ok = 0,
    Exited = 1,
    Timeout = 2,
    OutDated = 3,
    InvalidReq = -1,
    Internal = -2,
}

#[no_mangle]
pub unsafe extern "C" fn qni_vec_delete(vec: *mut QniVec) {
    let _ = ::std::ptr::read(vec).into_vec();
}

#[no_mangle]
pub unsafe extern "C" fn qni_wait(
    ctx: ConsoleArcCtx,
    buf: *mut u8,
    len: usize,
    out: *mut QniVec,
) -> QniWaitResult {
    let req = protobuf::parse_from_bytes(::std::slice::from_raw_parts(buf, len));

    match req {
        Ok(req) => match (*ctx).wait_console(req) {
            Ok(res) => match protobuf::Message::write_to_bytes(&*res) {
                Ok(buf) => {
                    *out = QniVec::from_vec(buf);
                    QniWaitResult::Ok
                }
                Err(_) => QniWaitResult::Internal,
            },
            Err(WaitError::Exited) => QniWaitResult::Exited,
            Err(WaitError::Timeout) => QniWaitResult::Timeout,
            Err(WaitError::OutDated) => QniWaitResult::OutDated,
        },
        _ => QniWaitResult::InvalidReq,
    }
}
