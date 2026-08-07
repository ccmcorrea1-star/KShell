use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use image::{
    imageops::{self, FilterType},
    DynamicImage, ImageReader, Rgba, RgbaImage,
};
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui_image::picker::{Capability, Picker, ProtocolType};
use ratatui_image::{protocol::Protocol, Resize};

const ICON_PIXEL_SIZE: f32 = 64.0;
const ICON_TARGET_SIZE: u32 = 32;
pub const ICON_CELL_SIZE: Size = Size::new(4, 2);

pub struct IconCache {
    picker: Picker,
    diagnostics: PickerDiagnostics,
    resolver: IconResolver,
    resolved: HashMap<String, Option<PathBuf>>,
    images: HashMap<PathBuf, Option<DynamicImage>>,
    protocols: HashMap<ProtocolKey, Option<Protocol>>,
    geometry: IconGeometry,
}

impl IconCache {
    pub fn new(picker: Picker, diagnostics: PickerDiagnostics) -> Self {
        let geometry = IconGeometry::new(picker.font_size());
        Self {
            picker,
            diagnostics,
            resolver: IconResolver::new(),
            resolved: HashMap::new(),
            images: HashMap::new(),
            geometry,
            protocols: HashMap::new(),
        }
    }

    pub fn protocol_for(&mut self, icon_name: Option<&str>) -> Option<&Protocol> {
        let icon_name = icon_name?.trim();
        if icon_name.is_empty() {
            return None;
        }

        if !self.resolved.contains_key(icon_name) {
            let path = self.resolver.resolve(icon_name);
            self.resolved.insert(icon_name.to_owned(), path);
        }

        let path = self.resolved.get(icon_name)?.as_ref()?.clone();
        let key = ProtocolKey {
            path: path.clone(),
            geometry: self.geometry,
        };

        if !self.protocols.contains_key(&key) {
            let protocol = self.protocol_for_path(&path);
            self.protocols.insert(key.clone(), protocol);
        }

        self.protocols.get(&key).and_then(Option::as_ref)
    }

    pub fn diagnostics(&self) -> &PickerDiagnostics {
        &self.diagnostics
    }

    pub fn on_resize(&mut self) {
        let geometry = IconGeometry::new(self.picker.font_size());
        if geometry != self.geometry {
            self.geometry = geometry;
            self.protocols.clear();
        }
    }

    fn image_for(&mut self, path: &Path) -> Option<&DynamicImage> {
        if !self.images.contains_key(path) {
            self.images.insert(path.to_owned(), load_image(path));
        }

        self.images.get(path)?.as_ref()
    }

    fn protocol_for_path(&mut self, path: &Path) -> Option<Protocol> {
        let image = self.image_for(path)?.clone();
        let canvas = image_to_canvas(
            &image,
            self.geometry.pixel_width,
            self.geometry.pixel_height,
        )?;
        let area = Rect::new(0, 0, self.geometry.cells.width, self.geometry.cells.height);

        self.picker
            .new_protocol(canvas, area, Resize::Fit(None))
            .ok()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct IconGeometry {
    cell_size: (u16, u16),
    cells: Size,
    pixel_width: u32,
    pixel_height: u32,
}

impl IconGeometry {
    fn new(cell_size: (u16, u16)) -> Self {
        let pixel_width = u32::from(ICON_CELL_SIZE.width) * u32::from(cell_size.0);
        let pixel_height = u32::from(ICON_CELL_SIZE.height) * u32::from(cell_size.1);
        Self {
            cell_size,
            cells: ICON_CELL_SIZE,
            pixel_width,
            pixel_height,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ProtocolKey {
    path: PathBuf,
    geometry: IconGeometry,
}

fn image_to_canvas(
    image: &DynamicImage,
    pixel_width: u32,
    pixel_height: u32,
) -> Option<DynamicImage> {
    if pixel_width == 0 || pixel_height == 0 {
        return None;
    }

    let resized = image
        .resize(pixel_width, pixel_height, FilterType::Lanczos3)
        .to_rgba8();
    let x = i64::from(pixel_width.saturating_sub(resized.width()) / 2);
    let y = i64::from(pixel_height.saturating_sub(resized.height()) / 2);
    let mut canvas = RgbaImage::from_pixel(pixel_width, pixel_height, Rgba([0, 0, 0, 0]));
    imageops::overlay(&mut canvas, &resized, x, y);
    Some(DynamicImage::ImageRgba8(canvas))
}

#[derive(Debug, Clone)]
pub struct PickerDiagnostics {
    pub protocol: ProtocolType,
    pub cell_size: (u16, u16),
    pub capabilities: Vec<Capability>,
    pub query_result: String,
    pub term: Option<String>,
    pub term_program: Option<String>,
}

pub fn detect_picker() -> (Picker, PickerDiagnostics) {
    // The query must happen after raw mode is enabled and before reading
    // events. The caller performs this before creating the TUI session so the
    // terminal capability query cannot interfere with Ratatui's first frame.
    let term = env::var("TERM").ok();
    let term_program = env::var("TERM_PROGRAM").ok();
    let result = Picker::from_query_stdio();
    let (picker, query_result) = match result {
        Ok(picker) => {
            let query_result = if picker.protocol_type() == ProtocolType::Halfblocks
                && picker.capabilities().is_empty()
            {
                "Ok(Halfblocks; ratatui-image internal capability fallback)".to_owned()
            } else {
                "Ok".to_owned()
            };
            (picker, query_result)
        }
        Err(error) => (Picker::halfblocks(), format!("Err({error:?})")),
    };

    let diagnostics = PickerDiagnostics {
        protocol: picker.protocol_type(),
        cell_size: picker.font_size(),
        capabilities: picker.capabilities().clone(),
        query_result,
        term,
        term_program,
    };

    (picker, diagnostics)
}

fn load_image(path: &Path) -> Option<DynamicImage> {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase());

    if matches!(extension.as_deref(), Some("svg" | "svgz")) {
        return rasterize_svg(path);
    }

    ImageReader::open(path).ok()?.decode().ok()
}

fn rasterize_svg(path: &Path) -> Option<DynamicImage> {
    let data = fs::read(path).ok()?;
    let options = resvg::usvg::Options {
        resources_dir: path.parent().map(Path::to_path_buf),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(&data, &options).ok()?;

    let size = tree.size();
    let largest_dimension = size.width().max(size.height());
    if !largest_dimension.is_finite() || largest_dimension <= 0.0 {
        return None;
    }

    let scale = ICON_PIXEL_SIZE / largest_dimension;
    let width = (size.width() * scale).round().max(1.0) as u32;
    let height = (size.height() * scale).round().max(1.0) as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let mut pixels = pixmap.data().to_vec();
    unpremultiply_alpha(&mut pixels);
    let image = RgbaImage::from_raw(width, height, pixels)?;
    Some(DynamicImage::ImageRgba8(image))
}

fn unpremultiply_alpha(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha == 0 || alpha == u16::from(u8::MAX) {
            continue;
        }

        for channel in &mut pixel[..3] {
            *channel = (u16::from(*channel) * u16::from(u8::MAX) / alpha).min(255) as u8;
        }
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    fn cache_with_test_images() -> IconCache {
        let picker = Picker::halfblocks();
        let diagnostics = PickerDiagnostics {
            protocol: picker.protocol_type(),
            cell_size: picker.font_size(),
            capabilities: Vec::new(),
            query_result: "test".to_owned(),
            term: None,
            term_program: None,
        };
        let mut cache = IconCache::new(picker, diagnostics);

        for (name, path, color) in [
            (
                "first-icon",
                PathBuf::from("first.png"),
                Rgba([255, 0, 0, 255]),
            ),
            (
                "second-icon",
                PathBuf::from("second.png"),
                Rgba([0, 255, 0, 255]),
            ),
        ] {
            cache.resolved.insert(name.to_owned(), Some(path.clone()));
            cache.images.insert(
                path,
                Some(DynamicImage::ImageRgba8(RgbaImage::from_pixel(3, 1, color))),
            );
        }

        cache
    }

    #[test]
    fn caches_multiple_fixed_protocols_without_resizing_the_original_image() {
        let mut cache = cache_with_test_images();

        let first_area = cache.protocol_for(Some("first-icon")).unwrap().area();
        let second_area = cache.protocol_for(Some("second-icon")).unwrap().area();
        let first_again_area = cache.protocol_for(Some("first-icon")).unwrap().area();

        assert_eq!(cache.protocols.len(), 2);
        assert_eq!(first_area, first_again_area);
        assert_eq!(second_area, Rect::new(0, 0, 4, 2));
        assert_eq!(
            cache.protocols[&ProtocolKey {
                path: PathBuf::from("first.png"),
                geometry: cache.geometry,
            }]
                .as_ref()
                .unwrap()
                .area(),
            Rect::new(0, 0, 4, 2)
        );
        assert_eq!(
            cache.images[&PathBuf::from("first.png")]
                .as_ref()
                .unwrap()
                .width(),
            3
        );
        assert_eq!(
            cache.images[&PathBuf::from("first.png")]
                .as_ref()
                .unwrap()
                .height(),
            1
        );
    }

    #[test]
    fn canvas_is_exactly_the_terminal_cell_geometry() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(3, 1, Rgba([1, 2, 3, 255])));
        let canvas = image_to_canvas(&image, 40, 40).unwrap();

        assert_eq!((canvas.width(), canvas.height()), (40, 40));
    }

    #[test]
    fn resize_invalidates_only_geometry_dependent_protocols() {
        let mut cache = cache_with_test_images();
        cache.protocol_for(Some("first-icon"));
        cache.protocol_for(Some("second-icon"));
        assert_eq!(cache.protocols.len(), 2);

        cache.geometry = IconGeometry::new((8, 16));
        cache.on_resize();

        assert!(cache.protocols.is_empty());
        assert_eq!(cache.geometry.pixel_width, 40);
        assert_eq!(cache.geometry.pixel_height, 40);
    }
}

struct IconResolver {
    roots: Vec<PathBuf>,
    themes: Vec<String>,
}

impl IconResolver {
    fn new() -> Self {
        Self {
            roots: icon_roots(),
            themes: icon_themes(),
        }
    }

    fn resolve(&self, icon_name: &str) -> Option<PathBuf> {
        let icon_path = Path::new(icon_name);
        if icon_path.is_absolute() {
            return icon_path.is_file().then(|| icon_path.to_owned());
        }

        for root in &self.roots {
            for theme in &self.themes {
                let mut visited = HashSet::new();
                if let Some(path) = find_in_theme(root, theme, icon_name, &mut visited) {
                    return Some(path);
                }
            }
        }

        self.roots
            .iter()
            .filter(|root| root.ends_with("pixmaps"))
            .find_map(|root| find_icon_file(root, icon_name))
    }
}

fn icon_roots() -> Vec<PathBuf> {
    let mut roots = Vec::with_capacity(8);
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.clone().map(|path| path.join(".local/share")));

    if let Some(data_home) = data_home {
        push_unique(&mut roots, data_home.join("icons"));
    }
    if let Some(home) = &home {
        push_unique(&mut roots, home.join(".icons"));
    }

    let data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|directories| env::split_paths(&directories).collect::<Vec<_>>())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });
    for data_dir in data_dirs.into_iter().filter(|path| path.is_absolute()) {
        push_unique(&mut roots, data_dir.join("icons"));
    }

    push_unique(&mut roots, PathBuf::from("/usr/local/share/icons"));
    push_unique(&mut roots, PathBuf::from("/usr/share/icons"));
    push_unique(&mut roots, PathBuf::from("/usr/local/share/pixmaps"));
    push_unique(&mut roots, PathBuf::from("/usr/share/pixmaps"));
    roots
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|current| current == &path) {
        paths.push(path);
    }
}

fn icon_themes() -> Vec<String> {
    let mut themes = Vec::with_capacity(3);

    if let Some(theme) = env::var_os("GTK_ICON_THEME") {
        push_theme(&mut themes, theme.to_string_lossy());
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        for path in [
            home.join(".config/gtk-4.0/settings.ini"),
            home.join(".config/gtk-3.0/settings.ini"),
            home.join(".gtkrc-2.0"),
        ] {
            if let Some(theme) = read_icon_theme_setting(&path) {
                push_theme(&mut themes, theme);
            }
        }
    }

    push_theme(&mut themes, "hicolor");
    themes
}

fn push_theme(themes: &mut Vec<String>, theme: impl AsRef<str>) {
    let theme = theme.as_ref().trim();
    if !theme.is_empty() && !themes.iter().any(|current| current == theme) {
        themes.push(theme.to_owned());
    }
}

fn read_icon_theme_setting(path: &Path) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "gtk-icon-theme-name").then(|| value.trim().trim_matches('"').to_owned())
    })
}

fn find_in_theme(
    root: &Path,
    theme: &str,
    icon_name: &str,
    visited: &mut HashSet<String>,
) -> Option<PathBuf> {
    if !visited.insert(theme.to_owned()) {
        return None;
    }

    let theme_dir = root.join(theme);
    if !theme_dir.is_dir() {
        return None;
    }

    let (directories, inherits) = read_theme_index(&theme_dir);
    let directories = if directories.is_empty() {
        vec![String::new()]
    } else {
        directories
    };

    let mut best = None;
    for directory in directories {
        let directory_path = theme_dir.join(&directory);
        if let Some(path) = find_icon_file(&directory_path, icon_name) {
            let score = directory_score(&directory);
            if best
                .as_ref()
                .is_none_or(|(best_score, _)| score < *best_score)
            {
                best = Some((score, path));
            }
        }
    }

    if let Some((_, path)) = best {
        return Some(path);
    }

    inherits
        .iter()
        .find_map(|inherit| find_in_theme(root, inherit, icon_name, visited))
}

fn read_theme_index(theme_dir: &Path) -> (Vec<String>, Vec<String>) {
    let Ok(contents) = fs::read_to_string(theme_dir.join("index.theme")) else {
        return (Vec::new(), Vec::new());
    };

    let mut in_icon_theme = false;
    let mut directories = Vec::new();
    let mut inherits = Vec::new();

    for line in contents.lines() {
        if let Some(section) = line
            .trim()
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            in_icon_theme = section == "Icon Theme";
            continue;
        }
        if !in_icon_theme {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "Directories" => {
                directories.extend(value.split(',').map(|item| item.trim().to_owned()))
            }
            "Inherits" => inherits.extend(value.split(',').map(|item| item.trim().to_owned())),
            _ => {}
        }
    }

    (directories, inherits)
}

fn directory_score(directory: &str) -> u32 {
    if directory.contains("scalable") {
        return 0;
    }

    let Some(size) = directory.split('/').find_map(parse_icon_size) else {
        return ICON_TARGET_SIZE;
    };
    size.abs_diff(ICON_TARGET_SIZE)
}

fn parse_icon_size(directory: &str) -> Option<u32> {
    let dimensions = directory.split('/').next()?;
    let (width, height) = dimensions.split_once('x')?;
    let width = width.parse::<u32>().ok()?;
    let height = height.parse::<u32>().ok()?;
    Some(width.max(height))
}

fn find_icon_file(directory: &Path, icon_name: &str) -> Option<PathBuf> {
    let name = Path::new(icon_name);
    let candidates = if name.extension().is_some() {
        vec![icon_name.to_owned()]
    } else {
        ["", ".png", ".svg", ".svgz", ".jpg", ".jpeg"]
            .iter()
            .map(|extension| format!("{icon_name}{extension}"))
            .collect()
    };

    candidates
        .into_iter()
        .map(|candidate| directory.join(candidate))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::{directory_score, parse_icon_size};

    #[test]
    fn prefers_scalable_and_nearby_icon_sizes() {
        assert_eq!(directory_score("scalable/apps"), 0);
        assert_eq!(directory_score("32x32/apps"), 0);
        assert_eq!(directory_score("16x16/apps"), 16);
    }

    #[test]
    fn parses_theme_directory_sizes() {
        assert_eq!(parse_icon_size("48x48/apps"), Some(48));
        assert_eq!(parse_icon_size("scalable/apps"), None);
    }
}
