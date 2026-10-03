use crate::model::{Result, MAX_IMAGE};
use image::{ImageFormat, ImageReader};
use std::io::Cursor;

/// The original stays encrypted on disk. Only this bounded PNG crosses IPC.
pub fn create(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() > MAX_IMAGE {
        return Err("Obraz przekracza limit podglądu.".into());
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_IMAGE as u64 * 4);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| "Nie można odczytać podglądu obrazu.".to_string())?;
    let preview = if image.width() > 512 || image.height() > 320 {
        image.thumbnail(512, 320)
    } else {
        image
    };
    let mut output = Cursor::new(Vec::new());
    preview
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| "Nie można przygotować podglądu obrazu.".to_string())?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba, RgbaImage};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(width, height, Rgba([12, 34, 56, 90])));
        let mut output = Cursor::new(Vec::new());
        image.write_to(&mut output, ImageFormat::Png).unwrap();
        output.into_inner()
    }

    #[test]
    fn bounded_preview_preserves_aspect_and_transparency_without_changing_original() {
        for (width, height) in [(1920, 1080), (600, 2400), (2000, 1), (1, 2000), (32, 16)] {
            let original = png(width, height);
            let before = original.clone();
            let preview = image::load_from_memory(&create(&original).unwrap()).unwrap();
            assert!(preview.width() <= 512 && preview.height() <= 320);
            assert!(preview.width() > 0 && preview.height() > 0);
            assert_eq!(preview.to_rgba8().get_pixel(0, 0).0, [12, 34, 56, 90]);
            assert_eq!(original, before);
            if width <= 512 && height <= 320 {
                assert_eq!((preview.width(), preview.height()), (width, height));
            }
        }
    }

    #[test]
    fn corrupt_or_oversized_input_is_rejected() {
        assert!(create(b"not a PNG").is_err());
        assert!(create(&vec![0; MAX_IMAGE + 1]).is_err());
        let original = png(64, 64);
        assert!(create(&original[..original.len() / 2]).is_err());
    }

    #[test]
    fn previews_work_after_restart_and_do_not_create_plaintext_files() {
        use crate::{model::Content, store::Store};
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        let original = png(900, 600);
        let mut store = Store::open(root.clone()).unwrap();
        let content = Content {
            title: "Test image".into(),
            width: 900,
            height: 600,
            ..Default::default()
        };
        store
            .capture(content.clone(), "image", Some(original.clone()))
            .unwrap();
        store
            .capture(content, "image", Some(original.clone()))
            .unwrap();
        let page = store.query("", "images", "", "recent", 0).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!((page.items[0].width, page.items[0].height), (900, 600));
        let id = page.items[0].id.clone();
        let preview = create(&store.image(&id).unwrap()).unwrap();
        drop(store);
        let mut reopened = Store::open(root.clone()).unwrap();
        assert_eq!(create(&reopened.image(&id).unwrap()).unwrap(), preview);
        assert_eq!(reopened.image(&id).unwrap(), original);
        let files: Vec<_> = std::fs::read_dir(root.join("images"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].extension().unwrap(), "bin");
        assert!(!std::fs::read(&files[0]).unwrap().starts_with(b"\x89PNG"));
        reopened.delete(&id).unwrap();
        assert!(reopened.image(&id).is_err());
        assert_eq!(std::fs::read_dir(root.join("images")).unwrap().count(), 0);
    }
}
