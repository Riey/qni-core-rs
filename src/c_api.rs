use crate::hub::*;
use crate::protos::qni_api::*;

use std::mem;
use std::slice;
use std::sync::{Arc, Mutex};

#[no_mangle]
pub unsafe extern "C" fn qni_hub_new(entry: ProgramEntryFuncPtr) -> SharedHubPtr {
    Box::into_raw(Box::new(Arc::new(Mutex::new(Hub::new(
        ProgramEntryCallback(entry),
    )))))
}

#[no_mangle]
pub unsafe extern "C" fn qni_hub_delete(hub: SharedHubPtr) {
    let _ = Box::from_raw(hub);
}

#[no_mangle]
pub unsafe extern "C" fn qni_hub_exit(hub: SharedHubPtr) {
    (*hub).lock().unwrap().set_exit();
}

#[no_mangle]
pub unsafe extern "C" fn qni_print(ctx: ProgramEntryCtxArg, text: *const u8, len: usize) {
    let mut command = ProgramCommand::new();
    let text = String::from_utf8_unchecked(Vec::from(slice::from_raw_parts(text, len)));
    command.mut_PRINT().set_PRINT(text);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_print_line(ctx: ProgramEntryCtxArg, text: *const u8, len: usize) {
    let mut command = ProgramCommand::new();
    let text = String::from_utf8_unchecked(Vec::from(slice::from_raw_parts(text, len)));
    command.mut_PRINT().set_PRINT_LINE(text);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_font(
    ctx: ProgramEntryCtxArg,
    font_family: *const u8,
    font_family_len: usize,
    font_size: f32,
    font_style: u32,
) {
    let mut command = ProgramCommand::new();

    let mut font = Font::new();

    font.set_font_family(String::from_utf8_unchecked(Vec::from(
        slice::from_raw_parts(font_family, font_family_len),
    )));
    font.set_font_size(font_size);
    font.set_font_style(font_style);

    command.mut_UPDATE_SETTING().set_FONT(font);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_text_align(ctx: ProgramEntryCtxArg, text_align: u32) {
    let mut command = ProgramCommand::new();
    command
        .mut_UPDATE_SETTING()
        .set_TEXT_ALIGN(mem::transmute(text_align as u8));

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_text_color(ctx: ProgramEntryCtxArg, color: u32) {
    let mut command = ProgramCommand::new();
    command.mut_UPDATE_SETTING().set_TEXT_COLOR(color);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_back_color(ctx: ProgramEntryCtxArg, color: u32) {
    let mut command = ProgramCommand::new();
    command.mut_UPDATE_SETTING().set_BACK_COLOR(color);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_highlight_color(ctx: ProgramEntryCtxArg, color: u32) {
    let mut command = ProgramCommand::new();
    command.mut_UPDATE_SETTING().set_HIGHLIGHT_COLOR(color);

    (*ctx).append_command(command);
}

#[no_mangle]
pub unsafe extern "C" fn qni_wait_int(ctx: ProgramEntryCtxArg) -> i32 {
    let mut ret = 0;
    let mut req = ProgramRequest::new();
    req.mut_INPUT().mut_INT();

    (*ctx).wait_console(req, |res| {
        if !res.has_OK_INPUT() {
            return false;
        }

        match res.take_OK_INPUT().data {
            Some(InputResponse_oneof_data::INT(num)) => {
                ret = num;
                true
            }
            _ => false,
        }
    });

    ret
}
