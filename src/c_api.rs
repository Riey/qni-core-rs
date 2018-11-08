use crate::console::WaitError;
use crate::hub::*;
use crate::protos::qni_api::*;

use std::mem;
use std::slice;
use std::sync::Arc;

#[no_mangle]
pub unsafe extern "C" fn qni_hub_new(entry: ProgramEntryFuncPtr) -> SharedHubPtr {
    Box::into_raw(Box::new(Arc::new(Hub::new(ProgramEntryCallback(entry)))))
}

#[no_mangle]
pub unsafe extern "C" fn qni_hub_delete(hub: SharedHubPtr) {
    let _ = Box::from_raw(hub);
}

#[no_mangle]
pub unsafe extern "C" fn qni_hub_exit(hub: SharedHubPtr) {
    (*hub).set_exit();
}

use std::str;

#[no_mangle]
pub unsafe extern "C" fn qni_print(ctx: ProgramEntryCtxArg, text: *const u8, len: usize) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
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
pub unsafe extern "C" fn qni_print_line(
    ctx: ProgramEntryCtxArg,
    text: *const u8,
    len: usize,
) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();
        let text = str::from_utf8_unchecked(slice::from_raw_parts(text, len));
        command.mut_PRINT().set_PRINT_LINE(text.into());

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_new_line(ctx: ProgramEntryCtxArg) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();
        command.mut_PRINT().mut_NEW_LINE();

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_delete_line(ctx: ProgramEntryCtxArg, count: u32) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();
        command.mut_PRINT().set_DELETE_LINE(count);

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_font(
    ctx: ProgramEntryCtxArg,
    font_family: *const u8,
    font_family_len: usize,
    font_size: f32,
    font_style: u32,
) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();

        let mut font = Font::new();

        font.set_font_family(
            str::from_utf8_unchecked(slice::from_raw_parts(font_family, font_family_len)).into(),
        );
        font.set_font_size(font_size);
        font.set_font_style(font_style);

        command.mut_UPDATE_SETTING().set_FONT(font);

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_text_align(ctx: ProgramEntryCtxArg, text_align: u32) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();

        command
            .mut_UPDATE_SETTING()
            .set_TEXT_ALIGN(mem::transmute(text_align as u8));

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_text_color(ctx: ProgramEntryCtxArg, color: u32) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();
        command.mut_UPDATE_SETTING().set_TEXT_COLOR(color);

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_back_color(ctx: ProgramEntryCtxArg, color: u32) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();
        command.mut_UPDATE_SETTING().set_BACK_COLOR(color);

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_set_highlight_color(ctx: ProgramEntryCtxArg, color: u32) -> i32 {
    if Arc::strong_count(&*ctx) <= 2 {
        -1
    } else {
        let mut command = ProgramCommand::new();
        command.mut_UPDATE_SETTING().set_HIGHLIGHT_COLOR(color);

        (*ctx).append_command(command);

        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn qni_wait_int(ctx: ProgramEntryCtxArg, ret: *mut i32) -> i32 {
    let mut req = ProgramRequest::new();
    req.mut_INPUT().mut_INT();

    match (*ctx).wait_console(
        req,
        || Arc::strong_count(&*ctx) <= 2,
        |res| {
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
        },
        None,
    ) {
        Ok(_) => 0,
        Err(WaitError::Exited) => -1,
        Err(WaitError::Timeout) => 1,
    }
}
