//! Explicit ASS wrapping for builds of libass without Unicode line breaking.
use std::collections::HashMap;

pub(crate) struct Measure {
    cache: HashMap<char, f32>,
    size: f32,
    #[cfg(windows)]
    dc: *mut std::ffi::c_void,
    #[cfg(windows)]
    font: *mut std::ffi::c_void,
    #[cfg(windows)]
    previous: *mut std::ffi::c_void,
}
#[cfg(windows)]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(dc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn DeleteDC(dc: *mut std::ffi::c_void) -> i32;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strikeout: u32,
        charset: u32,
        out_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> *mut std::ffi::c_void;
    fn SelectObject(
        dc: *mut std::ffi::c_void,
        object: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn DeleteObject(object: *mut std::ffi::c_void) -> i32;
    fn GetTextExtentPoint32W(
        dc: *mut std::ffi::c_void,
        text: *const u16,
        length: i32,
        size: *mut [i32; 2],
    ) -> i32;
}
impl Measure {
    pub(crate) fn new(font: &str, size: f32) -> Self {
        #[cfg(windows)]
        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            let family: Vec<u16> = font.encode_utf16().chain(Some(0)).collect();
            // ASS size is a font cell height, not the EM height selected by negative GDI sizes.
            // Measure at higher resolution to avoid per-character integer rounding.
            let face = CreateFontW(
                (size * 16.0).ceil() as i32,
                0,
                0,
                0,
                700, // Measure bold as well so preserved inline bold tags fit the same box.
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                family.as_ptr(),
            );
            let previous = if !dc.is_null() && !face.is_null() {
                SelectObject(dc, face)
            } else {
                std::ptr::null_mut()
            };
            return Self {
                cache: HashMap::new(),
                size,
                dc,
                font: face,
                previous,
            };
        }
        #[cfg(not(windows))]
        {
            let _ = font;
            Self {
                cache: HashMap::new(),
                size,
            }
        }
    }
    fn width(&mut self, c: char) -> f32 {
        if let Some(width) = self.cache.get(&c) {
            return *width;
        }
        let mut width = if c.is_ascii() {
            if c.is_whitespace() {
                self.size * 0.4
            } else {
                self.size * 0.8
            }
        } else {
            self.size
        };
        #[cfg(windows)]
        unsafe {
            if !self.dc.is_null() && !self.font.is_null() {
                let mut utf16 = [0u16; 2];
                let text = c.encode_utf16(&mut utf16);
                let mut extent = [0i32; 2];
                if GetTextExtentPoint32W(self.dc, text.as_ptr(), text.len() as i32, &mut extent)
                    != 0
                {
                    width = (extent[0] as f32 / 16.0).max(0.0);
                }
            }
        }
        // Small rasterization headroom; a 15% EM-based reserve unnecessarily narrowed CJK lines.
        let width = width * 1.03;
        self.cache.insert(c, width);
        width
    }
    pub(crate) fn wrap(&mut self, text: &str, width: f32) -> String {
        let mut result = String::new();
        let mut used = 0.0;
        let mut in_tag = false;
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '{' {
                in_tag = true;
            }
            if in_tag {
                result.push(c);
                if c == '}' {
                    in_tag = false;
                }
                i += 1;
                continue;
            }
            if c == '\\' && i + 1 < chars.len() && matches!(chars[i + 1], 'N' | 'n') {
                result.push_str("\\N");
                used = 0.0;
                i += 2;
                continue;
            }
            let (rendered, advance) = if c == '\\' && chars.get(i + 1) == Some(&'h') {
                (' ', 2)
            } else {
                (c, 1)
            };
            let measured = self.width(rendered);
            if used > 0.0 && used + measured > width {
                result.push_str("\\N");
                used = 0.0;
            }
            for c in &chars[i..i + advance] {
                result.push(*c);
            }
            used += measured;
            i += advance;
        }
        result
    }
}
#[cfg(windows)]
impl Drop for Measure {
    fn drop(&mut self) {
        unsafe {
            if !self.dc.is_null() {
                if !self.previous.is_null() {
                    SelectObject(self.dc, self.previous);
                }
                DeleteDC(self.dc);
            }
            if !self.font.is_null() {
                DeleteObject(self.font);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_unspaced_text_wraps_and_preserves_tags_and_characters() {
        let mut measure = Measure::new("Segoe UI", 20.0);
        let input = "{\\b1}这是一句没有空格的长字幕用于验证边距{\\b0}ABC";
        let wrapped = measure.wrap(input, 70.0);
        assert!(wrapped.contains("\\N"));
        assert_eq!(wrapped.replace("\\N", ""), input);
        assert!(wrapped.contains("{\\b1}"));
        assert!(wrapped.contains("{\\b0}"));
    }
}
