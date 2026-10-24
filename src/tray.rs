//! Gestione della System Tray su Windows e apertura dell'app come finestra standalone (Edge/Chrome App Mode).

use std::ptr::null_mut;

#[cfg(windows)]
#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[cfg(windows)]
#[repr(C)]
struct MSG {
    hwnd: *mut (),
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: POINT,
}

#[cfg(windows)]
#[repr(C)]
struct WNDCLASSEXW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: Option<unsafe extern "system" fn(*mut (), u32, usize, isize) -> isize>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: *mut (),
    h_icon: *mut (),
    h_cursor: *mut (),
    hbr_background: *mut (),
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: *mut (),
}

#[cfg(windows)]
#[repr(C)]
struct NOTIFYICONDATAW {
    cb_size: u32,
    h_wnd: *mut (),
    u_id: u32,
    u_flags: u32,
    u_callback_message: u32,
    h_icon: *mut (),
    sz_tip: [u16; 128],
    dw_state: u32,
    dw_state_mask: u32,
    sz_info: [u16; 256],
    u_timeout_or_version: u32,
    sz_info_title: [u16; 64],
    dw_info_flags: u32,
}

#[cfg(windows)]
const WM_APP: u32 = 0x8000;
#[cfg(windows)]
const WM_TRAYICON: u32 = WM_APP + 1;
#[cfg(windows)]
const WM_LBUTTONUP: isize = 0x0202;
#[cfg(windows)]
const WM_LBUTTONDBLCLK: isize = 0x0203;
#[cfg(windows)]
const WM_RBUTTONUP: isize = 0x0205;

#[cfg(windows)]
const NIM_ADD: u32 = 0x00000000;
#[cfg(windows)]
const NIM_DELETE: u32 = 0x00000002;
#[cfg(windows)]
const NIF_MESSAGE: u32 = 0x00000001;
#[cfg(windows)]
const NIF_ICON: u32 = 0x00000002;
#[cfg(windows)]
const NIF_TIP: u32 = 0x00000004;

#[cfg(windows)]
const MF_STRING: u32 = 0x00000000;
#[cfg(windows)]
const MF_SEPARATOR: u32 = 0x00000800;
#[cfg(windows)]
const TPM_RIGHTBUTTON: u32 = 0x0002;
#[cfg(windows)]
const TPM_BOTTOMALIGN: u32 = 0x0020;

#[cfg(windows)]
const ID_OPEN_APP: usize = 1001;
#[cfg(windows)]
const ID_CHECK_RENEW: usize = 1002;
#[cfg(windows)]
const ID_OPEN_BROWSER: usize = 1003;
#[cfg(windows)]
const ID_EXIT: usize = 1004;

#[cfg(windows)]
#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(dwMessage: u32, lpData: *const NOTIFYICONDATAW) -> i32;
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn DefWindowProcW(hWnd: *mut (), Msg: u32, wParam: usize, lParam: isize) -> isize;
    fn RegisterClassExW(lpWndClass: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        dwExStyle: u32,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: u32,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: *mut (),
        hMenu: *mut (),
        hInstance: *mut (),
        lpParam: *mut (),
    ) -> *mut ();
    fn LoadIconW(hInstance: *mut (), lpIconName: usize) -> *mut ();
    fn GetCursorPos(lpPoint: *mut POINT) -> i32;
    fn SetForegroundWindow(hWnd: *mut ()) -> i32;
    fn CreatePopupMenu() -> *mut ();
    fn AppendMenuW(hMenu: *mut (), uFlags: u32, uIDNewItem: usize, lpNewItem: *const u16) -> i32;
    fn TrackPopupMenu(hMenu: *mut (), uFlags: u32, x: i32, y: i32, nReserved: i32, hWnd: *mut (), prcRect: *const ()) -> i32;
    fn DestroyMenu(hMenu: *mut ()) -> i32;
    fn PostQuitMessage(nExitCode: i32);
    fn GetMessageW(lpMsg: *mut MSG, hWnd: *mut (), wMsgFilterMin: u32, wMsgFilterMax: u32) -> i32;
    fn TranslateMessage(lpMsg: *const MSG) -> i32;
    fn DispatchMessageW(lpMsg: *const MSG) -> isize;
}

/// Apre la finestra dell'applicazione standalone usando Microsoft Edge o Chrome in modalità app
/// (senza barra degli indirizzi, senza schede, come un'applicazione nativa).
pub fn open_app_window() {
    #[cfg(windows)]
    {
        // Edge è preinstallato su ogni Windows 10 e 11
        let res = std::process::Command::new("msedge.exe")
            .args(["--app=http://127.0.0.1:8787", "--window-size=680,860"])
            .spawn();

        if res.is_err() {
            // Prova con Chrome
            let chrome_res = std::process::Command::new("chrome.exe")
                .args(["--app=http://127.0.0.1:8787", "--window-size=680,860"])
                .spawn();
            if chrome_res.is_err() {
                // Ripiego su browser predefinito
                let _ = std::process::Command::new("cmd")
                    .args(["/c", "start", "http://127.0.0.1:8787"])
                    .spawn();
            }
        }
    }
}

pub fn open_in_browser() {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "http://127.0.0.1:8787"])
            .spawn();
    }
}

#[cfg(windows)]
unsafe extern "system" fn tray_wnd_proc(hwnd: *mut (), msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_TRAYICON => {
            if lparam == WM_LBUTTONUP || lparam == WM_LBUTTONDBLCLK {
                open_app_window();
            } else if lparam == WM_RBUTTONUP {
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);
                SetForegroundWindow(hwnd);

                let hmenu = CreatePopupMenu();
                let to_w = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };

                AppendMenuW(hmenu, MF_STRING, ID_OPEN_APP, to_w("Open AltKeeper").as_ptr());
                AppendMenuW(hmenu, MF_STRING, ID_CHECK_RENEW, to_w("Check & Renew Now").as_ptr());
                AppendMenuW(hmenu, MF_STRING, ID_OPEN_BROWSER, to_w("Open in Browser").as_ptr());
                AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
                AppendMenuW(hmenu, MF_STRING, ID_EXIT, to_w("Exit AltKeeper").as_ptr());

                let cmd = TrackPopupMenu(
                    hmenu,
                    TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | 0x0100, // TPM_RETURNCMD
                    pt.x,
                    pt.y,
                    0,
                    hwnd,
                    null_mut(),
                );
                DestroyMenu(hmenu);

                match cmd as usize {
                    ID_OPEN_APP => open_app_window(),
                    ID_CHECK_RENEW => {
                        tokio::spawn(async {
                            let mut say = |_: &str| {};
                            let _ = crate::cmd_renew(None, false, false, 7, &mut say).await;
                        });
                    }
                    ID_OPEN_BROWSER => open_in_browser(),
                    ID_EXIT => {
                        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                        nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                        nid.h_wnd = hwnd;
                        nid.u_id = 1;
                        Shell_NotifyIconW(NIM_DELETE, &nid);
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Avvia l'icona nella barra delle applicazioni (System Tray) accanto all'orologio di Windows.
pub fn start() {
    #[cfg(windows)]
    {
        std::thread::spawn(|| unsafe {
            let class_name: Vec<u16> = "AltKeeperTrayWindow\0".encode_utf16().collect();
            let mut wcx: WNDCLASSEXW = std::mem::zeroed();
            wcx.cb_size = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wcx.lpfn_wnd_proc = Some(tray_wnd_proc);
            wcx.lpsz_class_name = class_name.as_ptr();

            RegisterClassExW(&wcx);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            );

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cb_size = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.h_wnd = hwnd;
            nid.u_id = 1;
            nid.u_flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.u_callback_message = WM_TRAYICON;
            nid.h_icon = LoadIconW(null_mut(), 32512); // IDI_APPLICATION standard icon

            let tip = "AltKeeper - iPhone Sideload Manager\0";
            for (i, c) in tip.encode_utf16().take(127).enumerate() {
                nid.sz_tip[i] = c;
            }

            Shell_NotifyIconW(NIM_ADD, &nid);

            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
    }
}
