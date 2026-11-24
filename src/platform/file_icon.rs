use std::path::Path;

use egui::{Color32, Pos2, Rect, TextureHandle, TextureId, Vec2};

/// Paints the OS-assigned file icon for `file_name` centered inside `rect` at `size`.
/// Returns `false` when no native icon is available so callers can fall back to `Icon::FileText`.
pub fn paint(ui: &egui::Ui, file_name: &str, rect: Rect, size: f32) -> bool {
    let Some(id) = texture_id(ui.ctx(), file_name) else {
        return false;
    };
    let target = Rect::from_center_size(rect.center(), Vec2::splat(size));
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    ui.painter().image(id, target, uv, Color32::WHITE);
    true
}

fn texture_id(ctx: &egui::Context, file_name: &str) -> Option<TextureId> {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())?
        .to_ascii_lowercase();
    let cache_id = egui::Id::new(("win-file-icon", ext.as_str()));
    if let Some(cached) = ctx.data(|data| data.get_temp::<Option<TextureHandle>>(cache_id)) {
        return cached.map(|handle| handle.id());
    }
    let handle = load_icon_image(&ext).map(|image| {
        ctx.load_texture(
            format!("win-file-icon-{ext}"),
            image,
            egui::TextureOptions::LINEAR,
        )
    });
    let id = handle.as_ref().map(TextureHandle::id);
    ctx.data_mut(|data| data.insert_temp(cache_id, handle));
    id
}

#[cfg(not(windows))]
fn load_icon_image(_ext: &str) -> Option<egui::ColorImage> {
    None
}

#[cfg(windows)]
fn load_icon_image(ext: &str) -> Option<egui::ColorImage> {
    use std::mem::size_of;
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
        DeleteObject, GetDIBits, GetObjectW,
    };
    use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
    use windows::Win32::UI::Shell::{
        SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};
    use windows::core::PCWSTR;

    let probe: Vec<u16> = format!("file.{ext}\0").encode_utf16().collect();
    let mut shfi = SHFILEINFOW::default();
    // SAFETY: `probe` is NUL-terminated and `shfi` is valid for the size passed.
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(probe.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut shfi),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
        )
    };
    if ok == 0 || shfi.hIcon.is_invalid() {
        return None;
    }

    let hicon = shfi.hIcon;
    let mut icon_info = ICONINFO::default();
    // SAFETY: `hicon` is a valid icon handle returned by `SHGetFileInfoW`.
    let got_info = unsafe { GetIconInfo(hicon, &mut icon_info) }.is_ok();
    // SAFETY: `hicon` is no longer needed once `GetIconInfo` has copied its bitmaps.
    unsafe {
        let _ = DestroyIcon(hicon);
    }
    if !got_info {
        return None;
    }

    let result = if !icon_info.hbmColor.is_invalid() {
        let mut bmp = BITMAP::default();
        // SAFETY: `hbmColor` is a valid HBITMAP owned by `icon_info`.
        let got_bmp = unsafe {
            GetObjectW(
                icon_info.hbmColor.into(),
                size_of::<BITMAP>() as i32,
                Some((&mut bmp as *mut BITMAP).cast()),
            )
        };
        if got_bmp > 0 && bmp.bmWidth > 0 && bmp.bmHeight != 0 {
            let width = bmp.bmWidth as usize;
            let height = bmp.bmHeight.unsigned_abs() as usize;
            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; width * height * 4];
            // SAFETY: `CreateCompatibleDC(None)` creates a memory DC compatible with the screen;
            // `bgra` is sized to `width * height * 4` for 32bpp `GetDIBits`.
            let lines = unsafe {
                let hdc = CreateCompatibleDC(None);
                let lines = if !hdc.is_invalid() {
                    let lines = GetDIBits(
                        hdc,
                        icon_info.hbmColor,
                        0,
                        height as u32,
                        Some(bgra.as_mut_ptr().cast()),
                        &mut bmi,
                        DIB_RGB_COLORS,
                    );
                    let _ = DeleteDC(hdc);
                    lines
                } else {
                    0
                };
                lines
            };
            if lines > 0 {
                for px in bgra.chunks_exact_mut(4) {
                    px.swap(0, 2);
                }
                if bgra.chunks_exact(4).all(|px| px[3] == 0) {
                    for px in bgra.chunks_exact_mut(4) {
                        if (px[0] | px[1] | px[2]) != 0 {
                            px[3] = 255;
                        }
                    }
                }
                Some(egui::ColorImage::from_rgba_unmultiplied(
                    [width, height],
                    &bgra,
                ))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // SAFETY: `GetIconInfo` transfers ownership of `hbmColor` and `hbmMask` to the caller.
    unsafe {
        if !icon_info.hbmColor.is_invalid() {
            let _ = DeleteObject(icon_info.hbmColor.into());
        }
        if !icon_info.hbmMask.is_invalid() {
            let _ = DeleteObject(icon_info.hbmMask.into());
        }
    }

    result
}
