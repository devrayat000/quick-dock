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

/// Minimal window handle wrapping a raw Win32 HWND, for the `drag` crate.
struct HwndHandle(isize);

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

/// Start an OS file drag of `paths` from the given HWND.
/// MUST be called on the UI (winit) thread while the mouse button is still held: DoDragDrop requires
/// the STA-initialized window thread that holds the mouse capture. It runs a nested modal loop
/// (painting/cursor handled by the OS) and returns after the drop.
pub fn start_file_drag(hwnd: isize, paths: Vec<String>) {
    let file_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    DRAG_OUT_ACTIVE.store(true, Ordering::Relaxed);
    let handle = HwndHandle(hwnd);
    let _ = drag::start_drag(
        &handle,
        drag::DragItem::Files(file_paths),
        drag::Image::Raw(vec![]),
        |_result, _cursor| {},
        drag::Options::default(),
    );
    DRAG_OUT_ACTIVE.store(false, Ordering::Relaxed);

    // DoDragDrop swallows the mouse-up that ended the drag, so Slint's TouchArea never releases
    // its grab and all later clicks route back to this same card. Post a synthetic WM_LBUTTONUP
    // so Slint sees the release and clears the grab.
    unsafe {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_LBUTTONUP};
        PostMessageW(hwnd as HWND, WM_LBUTTONUP, 0, 0);
    }
}
