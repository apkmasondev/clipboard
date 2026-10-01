use crate::{model::*, privacy, AppState};
use clipboard_win::{formats, raw, Clipboard, Getter};
use image::{DynamicImage, ImageDecoder, ImageFormat};
use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicIsize, Ordering},
        mpsc, OnceLock,
    },
    thread,
    time::Duration,
};
use tauri::{Emitter, Manager};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{DataExchange::*, LibraryLoader::GetModuleHandleW, Registry::*, Threading::*},
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

#[cfg(test)]
#[path = "windows_tests.rs"]
mod tests;

static EVENTS: OnceLock<mpsc::SyncSender<u32>> = OnceLock::new();
static OWNER: AtomicIsize = AtomicIsize::new(0);
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn format(name: &str) -> u32 {
    unsafe { RegisterClipboardFormatW(wide(name).as_ptr()) }
}
pub fn foreground() -> (isize, u32) {
    unsafe {
        let h = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(h, &mut pid);
        (h as isize, pid)
    }
}
fn source() -> String {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(GetClipboardOwner(), &mut pid);
        if pid == 0 {
            return String::new();
        }
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if p.is_null() {
            return String::new();
        }
        let mut buf = vec![0u16; 32768];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(p);
        if ok == 0 {
            String::new()
        } else {
            String::from_utf16_lossy(&buf[..len as usize])
                .rsplit('\\')
                .next()
                .unwrap_or_default()
                .to_lowercase()
        }
    }
}
fn read_raw(id: u32, limit: usize) -> Result<Vec<u8>> {
    if !raw::is_format_avail(id) {
        return Ok(vec![]);
    }
    if raw::size(id).map_or(0, |n| n.get()) > limit {
        return Err("Element przekracza limit rozmiaru.".into());
    }
    let mut b = vec![];
    raw::get_vec(id, &mut b)
        .map_err(|e| format!("Nie można odczytać formatu schowka {id} (Windows: {e})."))?;
    Ok(b)
}
fn sensitive_format() -> bool {
    if [
        "ExcludeClipboardContentFromMonitorProcessing",
        "Clipboard Viewer Ignore",
        "SuperClipboard.OwnWrite",
    ]
    .iter()
    .any(|n| raw::is_format_avail(format(n)))
    {
        return true;
    }
    ["CanIncludeInClipboardHistory", "CanUploadToCloudClipboard"]
        .iter()
        .any(|n| {
            read_raw(format(n), 8)
                .ok()
                .is_some_and(|b| b.len() >= 4 && b[..4] == [0, 0, 0, 0])
        })
}
struct Captured {
    content: Content,
    kind: String,
    image: Option<Vec<u8>>,
    image_is_dib: bool,
}
fn read(settings: &Settings) -> Result<Option<Captured>> {
    if settings.paused {
        return Ok(None);
    }
    let _clip = (0..8)
        .find_map(|_| {
            let c = Clipboard::new();
            if c.is_err() {
                thread::sleep(Duration::from_millis(12));
            }
            c.ok()
        })
        .ok_or("Schowek jest zajęty przez inną aplikację.")?;
    let src = source();
    if privacy::blocked_source(settings, &src) || sensitive_format() {
        return Ok(None);
    }
    let mut c = Content {
        source: src,
        ..Default::default()
    };
    let mut image_bytes = None;
    let mut image_is_dib = false;
    let kind;
    if raw::is_format_avail(formats::CF_HDROP) {
        if settings.disabled_types.iter().any(|t| t == "files") {
            return Ok(None);
        }
        if raw::size(formats::CF_HDROP).map_or(0, |n| n.get()) > MAX_TEXT {
            return Err("Lista plików jest zbyt duża.".into());
        }
        formats::FileList
            .read_clipboard(&mut c.files)
            .map_err(|_| "Nie można odczytać listy plików.")?;
        c.text = c.files.join("\n");
        kind = "files".to_string();
    } else if raw::is_format_avail(formats::CF_UNICODETEXT) {
        let b = read_raw(formats::CF_UNICODETEXT, MAX_TEXT * 2)?;
        let units: Vec<u16> = b
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .take_while(|u| *u != 0)
            .collect();
        c.text = String::from_utf16_lossy(&units);
        if c.text.is_empty() {
            return Ok(None);
        }
        if c.text.len() > MAX_TEXT {
            return Err("Tekst przekracza limit 4 MB.".into());
        }
        if settings.skip_secrets && privacy::likely_secret(&c.text) {
            return Ok(None);
        }
        c.html = read_raw(format("HTML Format"), MAX_TEXT)?;
        c.rtf = read_raw(format("Rich Text Format"), MAX_TEXT)?;
        kind = classify(&c.text, !c.html.is_empty() || !c.rtf.is_empty()).to_string();
    } else if raw::is_format_avail(format("PNG"))
        || raw::is_format_avail(formats::CF_DIB)
        || raw::is_format_avail(formats::CF_DIBV5)
    {
        if settings.disabled_types.iter().any(|t| t == "image") {
            return Ok(None);
        }
        let id = if raw::is_format_avail(format("PNG")) {
            format("PNG")
        } else if raw::is_format_avail(formats::CF_DIBV5) {
            image_is_dib = true;
            formats::CF_DIBV5
        } else {
            image_is_dib = true;
            formats::CF_DIB
        };
        image_bytes = Some(read_raw(id, MAX_IMAGE * 4)?);
        kind = "image".to_string();
    } else {
        return Ok(None);
    }
    // Release the clipboard before compression, encryption and database work.
    drop(_clip);
    if settings.disabled_types.contains(&kind) {
        return Ok(None);
    }
    Ok(Some(Captured {
        content: c,
        kind,
        image: image_bytes,
        image_is_dib,
    }))
}
fn read_retry(settings: &Settings) -> Result<Option<Captured>> {
    let mut last = read(settings);
    for delay in [20, 50, 100, 200] {
        if last.is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(delay));
        last = read(settings);
    }
    last
}
fn prepare(mut captured: Captured) -> Result<Captured> {
    let c = &mut captured.content;
    if let Some(bytes) = captured.image.take() {
        let img = if captured.image_is_dib {
            let bmp = dib_to_bmp(&bytes)?;
            let decoder = image::codecs::bmp::BmpDecoder::new(Cursor::new(&bmp))
                .map_err(|_| "Nieobsługiwany obraz DIB.")?;
            if decoder.total_bytes() > MAX_IMAGE as u64 * 4 {
                return Err("Obraz jest za duży (maks. 96 MB po dekodowaniu).".into());
            }
            DynamicImage::from_decoder(decoder)
                .map_err(|e| format!("Nie można odczytać obrazu: {e}"))?
        } else {
            let mut reader = image::ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png);
            let mut limits = image::Limits::default();
            limits.max_alloc = Some(MAX_IMAGE as u64 * 4);
            reader.limits(limits);
            reader
                .decode()
                .map_err(|_| "Nie można odczytać PNG lub obraz jest zbyt duży.")?
        };
        c.width = img.width();
        c.height = img.height();
        c.title = format!("Obraz · {} × {}", c.width, c.height);
        let mut out = Cursor::new(vec![]);
        img.write_to(&mut out, ImageFormat::Png)
            .map_err(|_| "Nie można zapisać PNG.")?;
        if out.get_ref().len() > MAX_IMAGE {
            return Err("Skompresowany obraz przekracza limit 24 MB.".into());
        }
        captured.image = Some(out.into_inner());
    } else {
        c.title = c
            .text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("Bez tytułu")
            .chars()
            .take(100)
            .collect();
    }
    Ok(captured)
}
// DIB has no file header. Compute its pixel offset explicitly: V4/V5 masks live
// inside the header, whereas a 40-byte INFOHEADER stores masks after it.
fn dib_to_bmp(dib: &[u8]) -> Result<Vec<u8>> {
    let u32_at = |i: usize| -> Result<u32> {
        let b = dib.get(i..i + 4).ok_or("Niepełny nagłówek DIB.")?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let header = u32_at(0)? as usize;
    if ![12, 40, 52, 56, 108, 124].contains(&header) || dib.len() < header {
        return Err("Nieobsługiwany nagłówek DIB.".into());
    }
    let bit_offset = if header == 12 { 10 } else { 14 };
    let bits = u16::from_le_bytes([dib[bit_offset], dib[bit_offset + 1]]);
    let used = if header == 12 {
        0
    } else {
        u32_at(32)? as usize
    };
    let colors = if used > 0 {
        used
    } else if bits <= 8 {
        1usize << bits
    } else {
        0
    };
    let masks = if header == 40 {
        match u32_at(16)? {
            3 => 12,
            6 => 16,
            _ => 0,
        }
    } else {
        0
    };
    let offset = header
        .checked_add(masks)
        .and_then(|n| {
            colors
                .checked_mul(if header == 12 { 3 } else { 4 })
                .and_then(|p| n.checked_add(p))
        })
        .ok_or("Nieprawidłowy rozmiar DIB.")?;
    if offset > dib.len() || dib.len() > MAX_IMAGE * 4 {
        return Err("Nieprawidłowy lub zbyt duży obraz DIB.".into());
    }
    let mut bmp = Vec::with_capacity(dib.len() + 14);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((dib.len() + 14) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&((offset + 14) as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    Ok(bmp)
}
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        if let Some(tx) = EVENTS.get() {
            let _ = tx.try_send(GetClipboardSequenceNumber());
        }
        return 0;
    }
    DefWindowProcW(hwnd, msg, w, l)
}
pub fn start_listener(app: tauri::AppHandle) -> Result<()> {
    let (tx, rx) = mpsc::sync_channel(128);
    EVENTS
        .set(tx)
        .map_err(|_| "Nasłuchiwanie schowka jest już aktywne.")?;
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::Builder::new()
        .name("clipboard-events".into())
        .spawn(move || unsafe {
            let name = wide("SuperClipboardListener");
            let instance = GetModuleHandleW(std::ptr::null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                ..std::mem::zeroed()
            };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                0,
                name.as_ptr(),
                name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() || AddClipboardFormatListener(hwnd) == 0 {
                let _ = ready_tx.send(false);
                return;
            }
            OWNER.store(hwnd as isize, Ordering::SeqCst);
            let _ = ready_tx.send(true);
            let mut msg = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            RemoveClipboardFormatListener(hwnd);
            DestroyWindow(hwnd);
        })
        .map_err(|_| "Nie można uruchomić nasłuchiwania schowka.")?;
    if !ready_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or(false)
    {
        return Err("Windows nie uruchomił nasłuchiwania schowka.".into());
    }
    // A bounded snapshot queue prevents image encoding from delaying the next read.
    let (capture_tx, capture_rx) = mpsc::sync_channel::<Captured>(4);
    let storage_app = app.clone();
    thread::Builder::new()
        .name("clipboard-storage".into())
        .spawn(move || {
            loop {
                match capture_rx.recv_timeout(Duration::from_secs(60)) {
                    Ok(c) => {
                        let result = prepare(c).and_then(|c| {
                            let state = storage_app.state::<AppState>();
                            let mut s = state
                                .store
                                .lock()
                                .map_err(|_| "Błąd magazynu.".to_string())?;
                            // Re-check privacy after queued work: pausing must take effect immediately.
                            if privacy::blocked_source(&s.settings, &c.content.source)
                                || s.settings.disabled_types.contains(&c.kind)
                                || (s.settings.skip_secrets
                                    && privacy::likely_secret(&c.content.text))
                            {
                                return Ok(false);
                            }
                            s.capture(c.content, &c.kind, c.image)
                        });
                        match result {
                            Ok(true) => {
                                let _ = storage_app.emit("changed", ());
                            }
                            Ok(false) => {}
                            Err(e) => {
                                log::error!("Clipboard persistence failed");
                                let _ = storage_app.emit("notice", e);
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if let Ok(mut s) = storage_app.state::<AppState>().store.lock() {
                            if s.cleanup().unwrap_or(0) > 0 {
                                let _ = storage_app.emit("changed", ());
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(|_| "Nie można uruchomić magazynu schowka.")?;
    thread::Builder::new().name("clipboard-capture".into()).spawn(move||{
        let mut previous=0;
        loop{
            match rx.recv_timeout(Duration::from_secs(60)){
                Ok(_)=>{
                    while rx.try_recv().is_ok(){}
                    let sequence=unsafe{GetClipboardSequenceNumber()};if sequence==previous{continue;}
                    let settings=match app.state::<AppState>().store.lock(){Ok(s)=>s.settings.clone(),Err(_)=>break};
                    match read_retry(&settings){
                        Ok(Some(c))=>{
                            previous=sequence;
                            if capture_tx.try_send(c).is_err(){let _=app.emit("notice","Schowek zmienia się zbyt szybko. Jeden element pominięto, aby ograniczyć zużycie pamięci.");}
                        },Ok(None)=>{previous=sequence;},Err(e)=>{previous=sequence;log::warn!("Clipboard capture skipped: {e}");let _=app.emit("notice",e);}
                    }
                },Err(mpsc::RecvTimeoutError::Timeout)=>{},Err(_)=>break
            }
        }
    }).map_err(|_|"Nie można uruchomić zapisu schowka.")?;
    Ok(())
}
pub fn write(e: &Entry, image: Option<Vec<u8>>, plain: Option<&str>) -> Result<()> {
    // Prepare all representations before clearing so conversion errors preserve the clipboard.
    let bitmap = if let Some(ref png) = image {
        let img = image::load_from_memory_with_format(png, ImageFormat::Png)
            .map_err(|_| "Nie można odczytać obrazu.")?;
        let mut b = Cursor::new(vec![]);
        img.to_rgba8()
            .write_to(&mut b, ImageFormat::Bmp)
            .map_err(|_| "Nie można przygotować obrazu.")?;
        Some(b.into_inner())
    } else {
        None
    };
    // Decode before taking the system-wide clipboard lock.
    let _clip = (0..8)
        .find_map(|_| {
            let c = Clipboard::new_for(OWNER.load(Ordering::SeqCst) as _);
            if c.is_err() {
                thread::sleep(Duration::from_millis(15));
            }
            c.ok()
        })
        .ok_or("Schowek jest zajęty. Spróbuj ponownie.")?;
    let fail = |_| "Windows nie pozwala zapisać do schowka.".to_string();
    raw::empty().map_err(fail)?;
    raw::set_without_clear(format("SuperClipboard.OwnWrite"), &[1]).map_err(fail)?;
    raw::set_without_clear(format("CanIncludeInClipboardHistory"), &[0, 0, 0, 0]).map_err(fail)?;
    raw::set_without_clear(format("CanUploadToCloudClipboard"), &[0, 0, 0, 0]).map_err(fail)?;
    if let Some(png) = image {
        raw::set_without_clear(format("PNG"), &png).map_err(fail)?;
        raw::set_without_clear(formats::CF_DIB, &bitmap.ok_or("Brak danych obrazu.")?[14..])
            .map_err(fail)?;
    } else if !e.content.files.is_empty() && plain.is_none() {
        raw::set_file_list_with(&e.content.files, clipboard_win::options::NoClear).map_err(fail)?;
    } else {
        raw::set_string_with(
            plain.unwrap_or(&e.content.text),
            clipboard_win::options::NoClear,
        )
        .map_err(fail)?;
        if plain.is_none() {
            if !e.content.html.is_empty() {
                raw::set_without_clear(format("HTML Format"), &e.content.html).map_err(fail)?;
            }
            if !e.content.rtf.is_empty() {
                raw::set_without_clear(format("Rich Text Format"), &e.content.rtf).map_err(fail)?;
            }
        }
    }
    Ok(())
}
pub fn paste(target: (isize, u32)) -> Result<()> {
    unsafe {
        let hwnd = target.0 as HWND;
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if hwnd.is_null() || IsWindow(hwnd) == 0 || pid != target.1 || pid == GetCurrentProcessId()
        {
            return Err(
                "Brak okna docelowego. Element skopiowano — użyj Ctrl+V w wybranej aplikacji."
                    .into(),
            );
        }
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
        // Activation across input queues is asynchronous. Wait for Windows to
        // acknowledge the target, without repeatedly stealing focus from the user.
        for _ in 0..30 {
            if GetForegroundWindow() == hwnd {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        for _ in 0..150 {
            if [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN, VK_RETURN]
                .iter()
                .all(|k| GetAsyncKeyState(*k as i32) >= 0)
            {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN, VK_RETURN]
            .iter()
            .any(|k| GetAsyncKeyState(*k as i32) < 0)
            || GetForegroundWindow() != hwnd
        {
            return Err(
                "Element skopiowano. Windows nie pozwolił odzyskać fokusu — użyj Ctrl+V.".into(),
            );
        }
        let input = |key, flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let inputs = [
            input(VK_CONTROL, 0),
            input(0x56, 0),
            input(0x56, KEYEVENTF_KEYUP),
            input(VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        if SendInput(4, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) != 4 {
            return Err("Element skopiowano, ale Windows zablokował automatyczne wklejenie (np. aplikacja administratora). Użyj Ctrl+V.".into());
        }
    }
    Ok(())
}
pub fn position_popup(window: &tauri::WebviewWindow, target: isize) -> Result<()> {
    unsafe {
        let monitor = MonitorFromWindow(target as HWND, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..std::mem::zeroed()
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return window.center().map_err(|e| e.to_string());
        }
        let r = info.rcWork;
        let scale = window
            .available_monitors()
            .map_err(|e| e.to_string())?
            .iter()
            .find(|m| m.position().x == info.rcMonitor.left && m.position().y == info.rcMonitor.top)
            .map(|m| m.scale_factor())
            .unwrap_or(1.0);
        let (x, y, width, height) =
            popup_bounds(r.left, r.top, r.right - r.left, r.bottom - r.top, scale);
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(tauri::PhysicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())
    }
}
fn popup_bounds(
    left: i32,
    top: i32,
    work_width: i32,
    work_height: i32,
    scale: f64,
) -> (i32, i32, u32, u32) {
    let width = ((640.0 * scale).round() as i32).min((work_width - 24).max(1));
    let height = ((520.0 * scale).round() as i32).min((work_height - 24).max(1));
    (
        left + (work_width - width) / 2,
        top + (work_height - height) / 3,
        width as u32,
        height as u32,
    )
}
pub fn autostart(enable: bool) -> Result<()> {
    unsafe {
        let mut key = std::ptr::null_mut();
        let path = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        ) != 0
        {
            return Err("Nie można zmienić autostartu użytkownika.".into());
        }
        let name = wide("SuperClipboard");
        let result = if enable {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let data = wide(&format!("\"{}\" --background", exe.display()));
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as _,
                (data.len() * 2) as u32,
            )
        } else {
            RegDeleteValueW(key, name.as_ptr())
        };
        RegCloseKey(key);
        if result != 0 && result != ERROR_FILE_NOT_FOUND {
            return Err("Nie można zapisać ustawienia autostartu.".into());
        }
    }
    Ok(())
}
