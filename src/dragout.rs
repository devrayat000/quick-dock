// Native drag-OUT via the `drag` crate (DoDragDrop) on the Slint window's raw HWND.
// No Tauri: we build a small Send wrapper exposing the HWND through raw-window-handle.

use std::num::NonZeroIsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};

/// Set true for the duration of an OLE drag-out so the edge poll does not auto-close mid-drag.
pub static DRAG_OUT_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Minimal Send-able window handle wrapping a raw Win32 HWND.
struct HwndHandle(isize);
unsafe impl Send for HwndHandle {}

impl HasWindowHandle for HwndHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd = NonZeroIsize::new(self.0).ok_or(HandleError::Unavailable)?;
        let handle = Win32WindowHandle::new(hwnd);
        // SAFETY: the HWND is valid for the lifetime of the app window.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

impl HasDisplayHandle for HwndHandle {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Windows has a single implicit display handle.
        Ok(unsafe { DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new())) })
    }
}

/// Start an OS file drag of `paths` from the given HWND. Runs on a background thread
/// (start_drag drives the blocking DoDragDrop loop) and self-clears DRAG_OUT_ACTIVE.
pub fn start_file_drag(hwnd: isize, paths: Vec<String>) {
    let file_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let exist: Vec<bool> = file_paths.iter().map(|p| p.exists()).collect();
    eprintln!(
        "[drag-out] start_file_drag hwnd={:#x} paths={:?} exists={:?}",
        hwnd, file_paths, exist
    );
    std::thread::spawn(move || {
        DRAG_OUT_ACTIVE.store(true, Ordering::Relaxed);
        let handle = HwndHandle(hwnd);
        let result = drag::start_drag(
            &handle,
            drag::DragItem::Files(file_paths),
            drag::Image::Raw(vec![]),
            |result, _cursor| {
                eprintln!("[drag-out] on_drop callback: {:?}", result);
            },
            drag::Options::default(),
        );
        match &result {
            Ok(_) => eprintln!("[drag-out] DoDragDrop returned Ok"),
            Err(e) => eprintln!("[drag-out] start_drag ERROR: {:?}", e),
        }
        DRAG_OUT_ACTIVE.store(false, Ordering::Relaxed);
    });
}
