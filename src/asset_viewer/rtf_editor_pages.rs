use super::{
    Editor, KillTimer, SendMessageW, SetTimer, SetWindowTextW, control_text, stream_in, tr, wide,
};

#[repr(C)]
#[derive(Default)]
struct Range {
    start: i32,
    end: i32,
}

fn selection(editor: &Editor<'_>) -> Range {
    let mut range = Range::default();
    unsafe {
        SendMessageW(editor.rich, 0x0434, 0, &mut range as *mut _ as isize);
    }
    range
}

fn select(editor: &Editor<'_>, start: i32, end: i32) {
    let range = Range { start, end };
    unsafe {
        SendMessageW(editor.rich, 0x0437, 0, &range as *const _ as isize);
    }
}

fn page_for(starts: &[i32], position: i32) -> usize {
    starts
        .partition_point(|&start| start <= position)
        .saturating_sub(1)
}

pub(super) fn schedule(editor: &Editor<'_>) {
    unsafe {
        SetTimer(editor.hwnd, 1, 250, std::ptr::null_mut());
    }
}

pub(super) fn refresh(editor: &Editor<'_>) {
    unsafe {
        KillTimer(editor.hwnd, 1);
    }
    let starts = super::super::native::page_starts(editor.rich);
    editor.page_starts.replace(starts);
    update_status(editor);
}

pub(super) fn update_status(editor: &Editor<'_>) {
    let position = selection(editor).end;
    let status = editor
        .page_starts
        .borrow()
        .as_ref()
        .map(|starts| format!("{} / {}", page_for(starts, position) + 1, starts.len()))
        .unwrap_or_else(|| "– / –".into());
    if control_text(editor.page_status) != status {
        unsafe {
            SetWindowTextW(editor.page_status, wide(&status).as_ptr());
        }
    }
}

pub(super) fn insert_break(editor: &Editor<'_>) -> Result<(), String> {
    let end = selection(editor).end;
    select(editor, end, end);
    stream_in(editor.rich, br"{\rtf1\page }", true)
}

pub(super) fn navigate(editor: &Editor<'_>, next: bool) -> Result<(), String> {
    refresh(editor);
    let position = selection(editor).end;
    let target = editor.page_starts.borrow().as_ref().and_then(|starts| {
        let page = page_for(starts, position);
        let target = if next {
            page.checked_add(1)
        } else {
            page.checked_sub(1)
        }?;
        starts.get(target).copied()
    });
    if let Some(target) = target {
        select(editor, target, target);
        unsafe {
            SendMessageW(editor.rich, 0x00b7, 0, 0);
        } // EM_SCROLLCARET
    }
    Ok(())
}

pub(super) fn delete(editor: &Editor<'_>) -> Result<(), String> {
    refresh(editor);
    let position = selection(editor).end;
    let Some((page, mut start, end)) = editor.page_starts.borrow().as_ref().map(|starts| {
        let page = page_for(starts, position);
        (
            page,
            starts[page],
            starts.get(page + 1).copied().unwrap_or(-1),
        )
    }) else {
        return Err(tr("无法确定当前页。").into());
    };
    let message = tr("删除第 {0} 页的内容？").replace("{0}", &(page + 1).to_string());
    if !super::style::confirm(editor, tr("删除当前页"), &message, tr("删除当前页")) {
        return Ok(());
    }

    // The last page can have an explicit break immediately before its first character.
    if end == -1 && start > 0 {
        #[repr(C)]
        struct TextRange {
            range: Range,
            text: *mut u16,
        }
        let mut text = [0u16; 8];
        let mut range = TextRange {
            range: Range {
                start: start - 1,
                end: start,
            },
            text: text.as_mut_ptr(),
        };
        unsafe {
            SendMessageW(editor.rich, 0x044b, 0, &mut range as *mut _ as isize);
        }
        if text[0] == 12 {
            start -= 1;
        }
    }
    select(editor, start, end);
    unsafe {
        SendMessageW(editor.rich, 0x00c2, 1, wide("").as_ptr() as isize);
    } // EM_REPLACESEL, undoable.
    refresh(editor);
    Ok(())
}
