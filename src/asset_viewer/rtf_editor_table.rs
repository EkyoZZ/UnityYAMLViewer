use super::{Handle, SendMessageW, control_text, tr};

#[repr(C)]
struct TableRow {
    size: u8,
    cell_size: u8,
    columns: u8,
    rows: u8,
    margin: i32,
    indent: i32,
    height: i32,
    flags: u32,
    start: i32,
    level: u8,
    cell_index: u8,
}

#[repr(C)]
struct TableCell {
    width: i32,
    flags: u16,
    shading: u16,
    borders: [i16; 4],
    colors: [u32; 6],
}

pub(super) fn insert(rich: Handle, inputs: [Handle; 3]) -> Result<(), String> {
    let [rows, columns, width] = inputs.map(|input| control_text(input).trim().parse::<u16>());
    let (Ok(rows @ 1..=50), Ok(columns @ 1..=12), Ok(width @ 20..=520)) = (rows, columns, width)
    else {
        return Err(tr("表格须为 1–50 行、1–12 列，列宽至少 20 pt，总宽最多 520 pt。").into());
    };
    if u32::from(columns) * u32::from(width) > 520 {
        return Err(tr("表格须为 1–50 行、1–12 列，列宽至少 20 pt，总宽最多 520 pt。").into());
    }
    let row = TableRow {
        size: std::mem::size_of::<TableRow>() as u8,
        cell_size: std::mem::size_of::<TableCell>() as u8,
        columns: columns as u8,
        rows: rows as u8,
        margin: 60,
        indent: 0,
        height: 0,
        flags: 1 | (1 << 7), // Left-aligned, one shared definition for identical cells.
        start: -1,
        level: 0,
        cell_index: 0,
    };
    let cell = TableCell {
        width: i32::from(width) * 20,
        flags: 0,
        shading: 0,
        borders: [10; 4],
        colors: [0, 0, 0, 0, 0xffffff, 0],
    };
    const EM_INSERTTABLE: u32 = 0x04e8;
    let status = unsafe {
        SendMessageW(
            rich,
            EM_INSERTTABLE,
            &row as *const _ as usize,
            &cell as *const _ as isize,
        )
    };
    if status != 0 {
        return Err(format!("RichEdit: 0x{:08X}", status as u32));
    }
    Ok(())
}
