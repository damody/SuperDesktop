//! Native Windows icon lookup with an owned RGBA boundary.

use std::{ffi::c_void, mem::size_of, os::windows::ffi::OsStrExt, path::Path, slice};

use shell_provider_protocol::IconData;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, HGDIOBJ, SelectObject,
        },
        Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
        UI::{
            Shell::{
                ExtractIconExW, SHFILEINFOW, SHGFI_ADDOVERLAYS, SHGFI_ICON, SHGFI_LARGEICON,
                SHGetFileInfoW,
            },
            WindowsAndMessaging::{
                DI_NORMAL, DestroyIcon, DrawIconEx, GCLP_HICON, GCLP_HICONSM, GetClassLongPtrW,
                HICON, ICON_BIG, ICON_SMALL, ICON_SMALL2, SMTO_ABORTIFHUNG, SendMessageTimeoutW,
                WM_GETICON,
            },
        },
    },
    core::PCWSTR,
};

const WINDOW_ICON_TIMEOUT_MS: u32 = 50;
const MAX_ICON_EDGE: u32 = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bc7Icon {
    pub width: u32,
    pub height: u32,
    pub padded_width: u32,
    pub padded_height: u32,
    pub row_pitch: u32,
    pub blocks: Vec<u8>,
}

pub fn valid_icon_data(icon: &IconData) -> bool {
    icon.width > 0
        && icon.height > 0
        && icon.width <= MAX_ICON_EDGE
        && icon.height <= MAX_ICON_EDGE
        && icon
            .width
            .checked_mul(icon.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .is_some_and(|length| length as usize == icon.rgba.len())
}

pub fn encode_bc7(icon: &IconData) -> Option<Bc7Icon> {
    if !valid_icon_data(icon) {
        return None;
    }
    let padded_width = icon.width.checked_add(3)? & !3;
    let padded_height = icon.height.checked_add(3)? & !3;
    let padded_stride = padded_width.checked_mul(4)? as usize;
    let mut padded = vec![0_u8; padded_stride.checked_mul(padded_height as usize)?];
    let source_stride = icon.width as usize * 4;
    for row in 0..icon.height as usize {
        let source = &icon.rgba[row * source_stride..(row + 1) * source_stride];
        let destination = &mut padded[row * padded_stride..(row + 1) * padded_stride];
        destination[..source_stride].copy_from_slice(source);
        if padded_width > icon.width {
            let edge = source[source_stride - 4..].to_vec();
            for column in icon.width as usize..padded_width as usize {
                destination[column * 4..column * 4 + 4].copy_from_slice(&edge);
            }
        }
    }
    if padded_height > icon.height {
        let last = padded
            [(icon.height as usize - 1) * padded_stride..icon.height as usize * padded_stride]
            .to_vec();
        for row in icon.height as usize..padded_height as usize {
            padded[row * padded_stride..(row + 1) * padded_stride].copy_from_slice(&last);
        }
    }
    let settings = if icon.rgba.chunks_exact(4).all(|pixel| pixel[3] == u8::MAX) {
        intel_tex_2::bc7::opaque_very_fast_settings()
    } else {
        intel_tex_2::bc7::alpha_very_fast_settings()
    };
    let blocks = intel_tex_2::bc7::compress_blocks(
        &settings,
        &intel_tex_2::RgbaSurface {
            data: &padded,
            width: padded_width,
            height: padded_height,
            stride: padded_stride as u32,
        },
    );
    let row_pitch = (padded_width / 4).checked_mul(16)?;
    let expected = row_pitch as usize * (padded_height as usize / 4);
    (blocks.len() == expected).then_some(Bc7Icon {
        width: icon.width,
        height: icon.height,
        padded_width,
        padded_height,
        row_pitch,
        blocks,
    })
}

/// Gets the Shell-owned icon for a filesystem item and returns owned pixels.
pub fn shell_icon_for_path(path: &Path, edge: u32) -> Option<IconData> {
    let edge = edge.clamp(1, MAX_ICON_EDGE);
    let wide = shell_compatible_path(path);
    for flags in [
        SHGFI_ICON | SHGFI_LARGEICON | SHGFI_ADDOVERLAYS,
        SHGFI_ICON | SHGFI_LARGEICON,
    ] {
        let mut info = SHFILEINFOW::default();
        // SAFETY: the path is NUL terminated, info is valid for the call, and a
        // successful SHGFI_ICON result transfers ownership of hIcon to us.
        let result = unsafe {
            SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut info),
                size_of::<SHFILEINFOW>() as u32,
                flags,
            )
        };
        if result == 0 || info.hIcon.is_invalid() {
            continue;
        }
        let pixels = icon_to_rgba(info.hIcon, edge);
        // SAFETY: SHGetFileInfoW returned this caller-owned icon exactly once.
        let _ = unsafe { DestroyIcon(info.hIcon) };
        if pixels.is_some() {
            return pixels;
        }
    }
    executable_resource_icon(path, &wide, edge)
}

fn executable_resource_icon(path: &Path, wide: &[u16], edge: u32) -> Option<IconData> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    let mut large = HICON::default();
    let mut small = HICON::default();
    // SAFETY: output slots are valid; successful handles are caller-owned.
    let extracted = unsafe {
        ExtractIconExW(
            PCWSTR(wide.as_ptr()),
            0,
            Some(&mut large),
            Some(&mut small),
            1,
        )
    };
    if extracted == 0 {
        return None;
    }
    let pixels = [large, small]
        .into_iter()
        .filter(|icon| !icon.is_invalid())
        .find_map(|icon| icon_to_rgba(icon, edge));
    if !large.is_invalid() {
        let _ = unsafe { DestroyIcon(large) };
    }
    if !small.is_invalid() && small != large {
        let _ = unsafe { DestroyIcon(small) };
    }
    pixels
}

fn shell_compatible_path(path: &Path) -> Vec<u16> {
    const EXTENDED_PREFIX: &[u16] = &[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    const UNC_PREFIX: &[u16] = &[
        b'\\' as u16,
        b'\\' as u16,
        b'?' as u16,
        b'\\' as u16,
        b'U' as u16,
        b'N' as u16,
        b'C' as u16,
        b'\\' as u16,
    ];
    let original = path.as_os_str().encode_wide().collect::<Vec<_>>();
    let mut result = if let Some(rest) = original.strip_prefix(UNC_PREFIX) {
        [b'\\' as u16, b'\\' as u16]
            .into_iter()
            .chain(rest.iter().copied())
            .collect()
    } else if let Some(rest) = original.strip_prefix(EXTENDED_PREFIX) {
        rest.to_vec()
    } else {
        original
    };
    result.push(0);
    result
}

/// Resolves a task icon from a live HWND and falls back to its executable.
pub fn window_icon(hwnd_identity: isize, executable: Option<&Path>, edge: u32) -> Option<IconData> {
    let hwnd = HWND(hwnd_identity as *mut c_void);
    for kind in [ICON_SMALL2, ICON_SMALL, ICON_BIG] {
        if let Some(icon) = borrowed_window_icon(hwnd, kind)
            && let Some(pixels) = icon_to_rgba(icon, edge)
        {
            return Some(pixels);
        }
    }
    // Class icons are borrowed from the registered window class.
    for index in [GCLP_HICONSM, GCLP_HICON] {
        // SAFETY: observation-only query; a zero result means no class icon.
        let raw = unsafe { GetClassLongPtrW(hwnd, index) };
        if raw != 0
            && let Some(pixels) = icon_to_rgba(HICON(raw as *mut c_void), edge)
        {
            return Some(pixels);
        }
    }
    executable.and_then(|path| shell_icon_for_path(path, edge))
}

fn borrowed_window_icon(hwnd: HWND, kind: u32) -> Option<HICON> {
    let mut raw = 0usize;
    // SAFETY: bounded synchronous message; returned HICON is borrowed from the window.
    let delivered = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            WPARAM(kind as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            WINDOW_ICON_TIMEOUT_MS,
            Some(&mut raw),
        )
    };
    (delivered.0 != 0 && raw != 0).then_some(HICON(raw as *mut c_void))
}

fn icon_to_rgba(icon: HICON, edge: u32) -> Option<IconData> {
    let edge = edge.clamp(1, MAX_ICON_EDGE);
    let byte_len = edge.checked_mul(edge)?.checked_mul(4)? as usize;
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: edge as i32,
            biHeight: -(edge as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: byte_len as u32,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };

    // SAFETY: every acquired GDI object is restored/released below before return.
    unsafe {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let mut bits = std::ptr::null_mut::<c_void>();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) => bitmap,
            Err(_) => {
                let _ = DeleteDC(dc);
                return None;
            }
        };
        let old = SelectObject(dc, HGDIOBJ(bitmap.0));
        let drawn = !old.is_invalid()
            && !bits.is_null()
            && DrawIconEx(dc, 0, 0, icon, edge as i32, edge as i32, 0, None, DI_NORMAL).is_ok();
        let mut rgba = drawn.then(|| slice::from_raw_parts(bits.cast::<u8>(), byte_len).to_vec());
        if !old.is_invalid() {
            let _ = SelectObject(dc, old);
        }
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);

        let rgba = rgba.as_mut()?;
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        if rgba.chunks_exact(4).all(|pixel| pixel[3] == 0) {
            for pixel in rgba.chunks_exact_mut(4) {
                if pixel[..3].iter().any(|channel| *channel != 0) {
                    pixel[3] = u8::MAX;
                }
            }
        }
        let output = IconData {
            width: edge,
            height: edge,
            rgba: std::mem::take(rgba),
        };
        valid_icon_data(&output).then_some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use windows::Win32::{
        System::Console::GetConsoleWindow,
        System::Threading::{GR_GDIOBJECTS, GetCurrentProcess, GetGuiResources},
    };

    static ICON_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn pixel_contract_rejects_malformed_buffers() {
        assert!(!valid_icon_data(&IconData {
            width: 32,
            height: 32,
            rgba: vec![0; 3],
        }));
    }

    #[test]
    fn extended_filesystem_paths_are_normalized_for_shell_apis() {
        let drive = shell_compatible_path(Path::new(r"\\?\C:\Users\Example\Item.lnk"));
        let unc = shell_compatible_path(Path::new(r"\\?\UNC\server\share\Item.lnk"));
        assert_eq!(
            String::from_utf16_lossy(&drive[..drive.len() - 1]),
            r"C:\Users\Example\Item.lnk"
        );
        assert_eq!(
            String::from_utf16_lossy(&unc[..unc.len() - 1]),
            r"\\server\share\Item.lnk"
        );
    }

    #[test]
    fn bc7_icon_blocks_are_four_to_one_and_preserve_alpha_mode() {
        let icon = IconData {
            width: 48,
            height: 48,
            rgba: vec![128; 48 * 48 * 4],
        };
        let encoded = encode_bc7(&icon).unwrap();
        assert_eq!((encoded.padded_width, encoded.padded_height), (48, 48));
        assert_eq!(encoded.row_pitch, 192);
        assert_eq!(encoded.blocks.len(), icon.rgba.len() / 4);
    }

    #[test]
    fn shell_executable_icon_has_owned_visible_pixels() {
        let _guard = ICON_TEST_LOCK.lock().unwrap();
        let icon = shell_icon_for_path(&std::env::current_exe().unwrap(), 32).unwrap();
        assert!(valid_icon_data(&icon));
        assert!(icon.rgba.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn repeated_shell_extraction_does_not_leak_gdi_objects() {
        let _guard = ICON_TEST_LOCK.lock().unwrap();
        let executable = std::env::current_exe().unwrap();
        // Warm the process-wide Shell/GDI icon cache before measuring resources
        // retained by Windows itself.
        assert!(shell_icon_for_path(&executable, 32).is_some());
        // SAFETY: query-only pseudo handle and process resource counter.
        let before = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        for _ in 0..64 {
            assert!(shell_icon_for_path(&executable, 32).is_some());
        }
        let after = unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) };
        assert!(
            after <= before.saturating_add(1),
            "GDI objects grew from {before} to {after}"
        );
    }

    #[test]
    fn window_lookup_falls_back_without_panicking() {
        let _guard = ICON_TEST_LOCK.lock().unwrap();
        // SAFETY: GetConsoleWindow is observation-only and may validly return null.
        let console = unsafe { GetConsoleWindow() };
        let executable = std::env::current_exe().unwrap();
        assert!(window_icon(console.0 as isize, Some(&executable), 24).is_some());
    }

    #[test]
    fn real_desktop_shell_items_have_visible_icons() {
        let _guard = ICON_TEST_LOCK.lock().unwrap();
        let entries = crate::common::desktop::enumerate_known_desktops().unwrap();
        let visible = entries
            .into_iter()
            .filter(|entry| !entry.hidden && !entry.system)
            .take(8)
            .collect::<Vec<_>>();
        assert!(!visible.is_empty());
        for entry in visible {
            assert!(
                shell_icon_for_path(&entry.canonical_path, 48).is_some(),
                "missing Shell icon for {}",
                entry.canonical_path.display()
            );
        }
    }
}
