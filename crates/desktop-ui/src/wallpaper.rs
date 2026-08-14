use std::collections::{BTreeMap, VecDeque};

use crate::LogicalRect;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WallpaperMode {
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
    Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Placement {
    SolidFallback,
    Single(LogicalRect),
    Tiles(Vec<LogicalRect>),
    Span {
        virtual_rect: LogicalRect,
        visible_rect: LogicalRect,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WallpaperError {
    InvalidImageSize,
    InvalidViewport,
    CacheItemTooLarge,
}

pub fn wallpaper_placement(
    mode: WallpaperMode,
    image: ImageSize,
    viewport: LogicalRect,
    virtual_rect: LogicalRect,
) -> Result<Placement, WallpaperError> {
    if image.width == 0 || image.height == 0 {
        return Err(WallpaperError::InvalidImageSize);
    }
    if viewport.width() <= 0.0 || viewport.height() <= 0.0 {
        return Err(WallpaperError::InvalidViewport);
    }
    let iw = image.width as f32;
    let ih = image.height as f32;
    let vw = viewport.width();
    let vh = viewport.height();
    let centered = |width: f32, height: f32| LogicalRect {
        left: viewport.left + (vw - width) / 2.0,
        top: viewport.top + (vh - height) / 2.0,
        right: viewport.left + (vw + width) / 2.0,
        bottom: viewport.top + (vh + height) / 2.0,
    };
    Ok(match mode {
        WallpaperMode::Stretch => Placement::Single(viewport),
        WallpaperMode::Center => Placement::Single(centered(iw, ih)),
        WallpaperMode::Fit => {
            let scale = (vw / iw).min(vh / ih);
            Placement::Single(centered(iw * scale, ih * scale))
        }
        WallpaperMode::Fill => {
            let scale = (vw / iw).max(vh / ih);
            Placement::Single(centered(iw * scale, ih * scale))
        }
        WallpaperMode::Tile => {
            let mut tiles = Vec::new();
            let mut y = viewport.top;
            while y < viewport.bottom {
                let mut x = viewport.left;
                while x < viewport.right {
                    tiles.push(LogicalRect {
                        left: x,
                        top: y,
                        right: x + iw,
                        bottom: y + ih,
                    });
                    x += iw;
                }
                y += ih;
            }
            Placement::Tiles(tiles)
        }
        WallpaperMode::Span => Placement::Span {
            virtual_rect,
            visible_rect: viewport,
        },
    })
}

#[derive(Clone, Debug)]
struct CacheEntry<T> {
    value: T,
    bytes: usize,
}

#[derive(Clone, Debug)]
pub struct BoundedWallpaperCache<T> {
    max_entries: usize,
    max_bytes: usize,
    bytes: usize,
    entries: BTreeMap<String, CacheEntry<T>>,
    order: VecDeque<String>,
}

impl<T> BoundedWallpaperCache<T> {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        assert!(max_entries > 0 && max_bytes > 0);
        Self {
            max_entries,
            max_bytes,
            bytes: 0,
            entries: BTreeMap::new(),
            order: VecDeque::new(),
        }
    }
    pub fn insert(&mut self, key: String, value: T, bytes: usize) -> Result<(), WallpaperError> {
        if bytes > self.max_bytes {
            return Err(WallpaperError::CacheItemTooLarge);
        }
        self.invalidate(&key);
        while self.entries.len() >= self.max_entries
            || self.bytes.saturating_add(bytes) > self.max_bytes
        {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.bytes -= entry.bytes;
            }
        }
        self.bytes += bytes;
        self.order.push_back(key.clone());
        self.entries.insert(key, CacheEntry { value, bytes });
        Ok(())
    }
    pub fn get(&self, key: &str) -> Option<&T> {
        self.entries.get(key).map(|entry| &entry.value)
    }
    pub fn invalidate(&mut self, key: &str) {
        if let Some(entry) = self.entries.remove(key) {
            self.bytes -= entry.bytes;
            self.order.retain(|existing| existing != key);
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(w: f32, h: f32) -> LogicalRect {
        LogicalRect::new(0.0, 0.0, w, h).unwrap()
    }
    #[test]
    fn all_six_modes_have_deterministic_geometry() {
        let image = ImageSize {
            width: 100,
            height: 50,
        };
        let view = rect(200.0, 200.0);
        let virtual_rect = rect(400.0, 200.0);
        assert_eq!(
            wallpaper_placement(WallpaperMode::Stretch, image, view, virtual_rect).unwrap(),
            Placement::Single(view)
        );
        assert!(
            matches!(wallpaper_placement(WallpaperMode::Tile,image,view,virtual_rect).unwrap(),Placement::Tiles(ref tiles) if tiles.len()==8)
        );
        assert!(matches!(
            wallpaper_placement(WallpaperMode::Span, image, view, virtual_rect).unwrap(),
            Placement::Span { .. }
        ));
        for mode in [
            WallpaperMode::Fill,
            WallpaperMode::Fit,
            WallpaperMode::Center,
        ] {
            assert!(matches!(
                wallpaper_placement(mode, image, view, virtual_rect).unwrap(),
                Placement::Single(_)
            ));
        }
    }
    #[test]
    fn invalid_source_uses_semantic_fallback_and_cache_is_bounded() {
        assert_eq!(
            wallpaper_placement(
                WallpaperMode::Fill,
                ImageSize {
                    width: 0,
                    height: 1
                },
                rect(10.0, 10.0),
                rect(10.0, 10.0)
            ),
            Err(WallpaperError::InvalidImageSize)
        );
        let mut cache = BoundedWallpaperCache::new(2, 10);
        cache.insert("a".into(), 1, 4).unwrap();
        cache.insert("b".into(), 2, 4).unwrap();
        cache.insert("c".into(), 3, 4).unwrap();
        assert_eq!(cache.len(), 2);
        assert!(cache.bytes() <= 10);
        assert!(cache.get("a").is_none());
    }
}
