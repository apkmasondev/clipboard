use super::*;

#[test]
fn popup_bounds_across_monitors_and_dpi() {
    for (left, top, w, h, scale) in [
        (0, 0, 1920, 1040, 1.0),
        (-2560, 0, 2560, 1400, 1.5),
        (1920, -400, 3840, 2080, 2.0),
        (0, 0, 1280, 680, 2.0),
    ] {
        let (x, y, width, height) = popup_bounds(left, top, w, h, scale);
        assert!(x >= left && y >= top);
        assert!(x + width as i32 <= left + w);
        assert!(y + height as i32 <= top + h);
    }
}

#[test]
fn png_and_dib_normalize_to_png() {
    let image = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(32, 16, |x, y| {
        image::Rgba([x as u8, y as u8, 120, (x * 7) as u8])
    }));
    for format in [ImageFormat::Png, ImageFormat::Bmp] {
        let mut data = Cursor::new(vec![]);
        image.write_to(&mut data, format).unwrap();
        let bytes = if format == ImageFormat::Bmp {
            data.into_inner()[14..].to_vec()
        } else {
            data.into_inner()
        };
        let c = prepare(Captured {
            content: Content::default(),
            kind: "image".into(),
            image: Some(bytes),
            image_is_dib: format == ImageFormat::Bmp,
        })
        .unwrap();
        assert_eq!((c.content.width, c.content.height), (32, 16));
        let decoded = image::load_from_memory(&c.image.unwrap()).unwrap();
        assert_eq!(decoded.to_rgba8(), image.to_rgba8());
    }
}

#[test]
fn invalid_images_fail_without_panicking() {
    for dib in [false, true] {
        assert!(prepare(Captured {
            content: Content::default(),
            kind: "image".into(),
            image: Some(vec![0; 150]),
            image_is_dib: dib
        })
        .is_err());
    }
}

// Explicit integration test: owns a message-only window, writes only synthetic
// data and empties the clipboard on exit. Never run as part of ordinary unit tests.
#[test]
#[ignore = "Changes the Windows clipboard. Run explicitly with --ignored --test-threads=1."]
fn live_windows_clipboard_formats_privacy_and_burst() {
    struct Owner(HWND);
    impl Drop for Owner {
        fn drop(&mut self) {
            if let Ok(_clip) = Clipboard::new_for(self.0 as _) {
                let _ = raw::empty();
            }
            OWNER.store(0, Ordering::SeqCst);
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }
    let class = wide("STATIC");
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            wide("Super Clipboard synthetic test").as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        )
    };
    assert!(!hwnd.is_null());
    let _owner = Owner(hwnd);
    OWNER.store(hwnd as isize, Ordering::SeqCst);
    let write_text = |text: &str| {
        let _clip = Clipboard::new_attempts_for(hwnd as _, 20).unwrap();
        raw::empty().unwrap();
        raw::set_string_with(text, clipboard_win::options::NoClear).unwrap();
    };
    let settings = Settings::default();
    for (text, kind) in [
        ("Zażółć gęślą jaźń 🦀".to_string(), "text"),
        ("https://example.com/local-test".into(), "link"),
        ("{\"n\":1234567890123456789}".into(), "json"),
        ("x".repeat(2 * 1024 * 1024), "text"),
    ] {
        write_text(&text);
        let c = read_retry(&settings).unwrap().unwrap();
        assert_eq!(c.content.text, text);
        assert_eq!(c.kind, kind);
    }
    write_text("privacy synthetic test");
    let captured = read_retry(&settings).unwrap().unwrap();
    assert!(!captured.content.source.is_empty());
    let mut blocked = settings.clone();
    blocked.blocked_apps.push(captured.content.source);
    assert!(read_retry(&blocked).unwrap().is_none());
    let mut disabled = settings.clone();
    disabled.disabled_types.push("text".into());
    assert!(read_retry(&disabled).unwrap().is_none());
    {
        let _clip = Clipboard::new_for(hwnd as _).unwrap();
        raw::set_without_clear(format("CanIncludeInClipboardHistory"), &[0, 0, 0, 0]).unwrap();
    }
    assert!(read_retry(&settings).unwrap().is_none());
    write_text("ghp_SYNTHETIC_TEST_TOKEN_NOT_A_REAL_SECRET");
    assert!(read_retry(&settings).unwrap().is_none());
    write_text("Formatted synthetic text");
    let rtf = b"{\\rtf1\\ansi Formatted synthetic text}";
    {
        let _clip = Clipboard::new_for(hwnd as _).unwrap();
        raw::set_without_clear(format("Rich Text Format"), rtf).unwrap();
    }
    let c = read_retry(&settings).unwrap().unwrap();
    assert_eq!(c.kind, "rich");
    assert_eq!(c.content.rtf, rtf);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("synthetic.txt");
    std::fs::write(&file, b"test").unwrap();
    {
        let _clip = Clipboard::new_for(hwnd as _).unwrap();
        raw::empty().unwrap();
        raw::set_file_list_with(
            &[file.to_string_lossy().to_string()],
            clipboard_win::options::NoClear,
        )
        .unwrap();
    }
    let c = read_retry(&settings).unwrap().unwrap();
    assert_eq!(c.kind, "files");
    assert_eq!(c.content.files.len(), 1);
    let image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        8,
        8,
        image::Rgba([12, 34, 56, 128]),
    ));
    let mut png = Cursor::new(vec![]);
    image.write_to(&mut png, ImageFormat::Png).unwrap();
    {
        let _clip = Clipboard::new_for(hwnd as _).unwrap();
        raw::empty().unwrap();
        raw::set_without_clear(format("PNG"), png.get_ref()).unwrap();
    }
    let c = prepare(read_retry(&settings).unwrap().unwrap()).unwrap();
    assert_eq!((c.content.width, c.content.height), (8, 8));
    let e = Entry {
        id: uuid::Uuid::new_v4().to_string(),
        kind: "image".into(),
        created: now(),
        updated: now(),
        bytes: 0,
        pinned: false,
        library: false,
        content: c.content,
    };
    write(&e, c.image, None).unwrap();
    assert!(read_retry(&settings).unwrap().is_none());
    {
        let _clip = Clipboard::new_for(hwnd as _).unwrap();
        let dib = read_raw(formats::CF_DIB, MAX_IMAGE).unwrap();
        let decoded =
            image::load_from_memory_with_format(&dib_to_bmp(&dib).unwrap(), ImageFormat::Bmp)
                .unwrap();
        assert_eq!(decoded.to_rgba8(), image.to_rgba8());
    }
    let mut store = crate::store::Store::open(directory.path().join("store")).unwrap();
    for i in 0..100 {
        write_text(&format!("Synthetic burst {i}"));
        let c = prepare(read_retry(&settings).unwrap().unwrap()).unwrap();
        store.capture(c.content, &c.kind, c.image).unwrap();
    }
    assert_eq!(store.stats().unwrap().history, 100);
    let c = prepare(read_retry(&settings).unwrap().unwrap()).unwrap();
    store.capture(c.content, &c.kind, c.image).unwrap();
    assert_eq!(store.stats().unwrap().history, 100);
}
