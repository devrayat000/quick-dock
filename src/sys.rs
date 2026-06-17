// Thin Win32 helpers via windows-sys: clipboard text get/set + open URL.
// Replaces the Tauri clipboard-manager plugin and tauri-plugin-opener (smaller, no extra crates).

use std::iter::once;

use windows_sys::Win32::Foundation::{HGLOBAL, HWND};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Read UTF-16 clipboard text (CF_UNICODETEXT). Returns None if empty/unavailable.
pub fn get_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(0 as HWND) == 0 {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT as u32);
        let result = if handle.is_null() {
            None
        } else {
            let ptr = GlobalLock(handle as HGLOBAL) as *const u16;
            if ptr.is_null() {
                None
            } else {
                // Find NUL terminator.
                let mut len = 0usize;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(ptr, len);
                let s = String::from_utf16_lossy(slice);
                let _ = GlobalUnlock(handle as HGLOBAL);
                Some(s)
            }
        };
        CloseClipboard();
        result
    }
}

/// Write text to the clipboard as CF_UNICODETEXT.
pub fn set_clipboard_text(text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(once(0)).collect();
    let bytes = wide.len() * std::mem::size_of::<u16>();
    unsafe {
        if OpenClipboard(0 as HWND) == 0 {
            return;
        }
        EmptyClipboard();
        let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if !hmem.is_null() {
            let dst = GlobalLock(hmem) as *mut u16;
            if !dst.is_null() {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
                let _ = GlobalUnlock(hmem);
                // Ownership of hmem passes to the clipboard on success.
                if SetClipboardData(CF_UNICODETEXT as u32, hmem as _).is_null() {
                    // SetClipboardData failed — we still own hmem; leak is acceptable (rare).
                }
            }
        }
        CloseClipboard();
    }
}

/// Open a URL/path in the default handler via ShellExecuteW("open", ...).
pub fn open_url(url: &str) {
    let op: Vec<u16> = "open".encode_utf16().chain(once(0)).collect();
    let target: Vec<u16> = url.encode_utf16().chain(once(0)).collect();
    unsafe {
        ShellExecuteW(
            0 as HWND,
            op.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}
