//! Canonical visual tokens and rendering helpers for KShell consumers.
//!
//! The renderer is called by `kshell-theme-gen` and its tests, while launcher
//! processes only need the geometry constants at runtime.
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub const BG_CANVAS: &str = "#1d2021";
pub const BG_SURFACE: &str = "#282828";
pub const BG_ELEVATED: &str = "#32302f";
pub const BG_SELECTED: &str = "#3c3836";
pub const BORDER_STRUCTURAL: &str = "#665c54";
pub const BORDER_MUTED: &str = "#3c3836";
pub const TEXT_PRIMARY: &str = "#ebdbb2";
pub const TEXT_SECONDARY: &str = "#a89984";
pub const TEXT_DISABLED: &str = "#928374";
pub const BACKDROP_OPACITY: &str = "0.28";

pub const ERROR: &str = "#cc241d";
pub const SUCCESS: &str = "#98971a";
pub const WARNING: &str = "#d79921";
pub const INFORMATION: &str = "#458588";

pub const ANSI_BLACK: &str = "#282828";
pub const ANSI_RED: &str = "#cc241d";
pub const ANSI_GREEN: &str = "#98971a";
pub const ANSI_YELLOW: &str = "#d79921";
pub const ANSI_BLUE: &str = "#458588";
pub const ANSI_MAGENTA: &str = "#b16286";
pub const ANSI_CYAN: &str = "#689d6a";
pub const ANSI_WHITE: &str = "#a89984";
pub const ANSI_BRIGHT_BLACK: &str = "#928374";
pub const ANSI_BRIGHT_RED: &str = "#fb4934";
pub const ANSI_BRIGHT_GREEN: &str = "#b8bb26";
pub const ANSI_BRIGHT_YELLOW: &str = "#fabd2f";
pub const ANSI_BRIGHT_BLUE: &str = "#83a598";
pub const ANSI_BRIGHT_MAGENTA: &str = "#d3869b";
pub const ANSI_BRIGHT_CYAN: &str = "#8ec07c";
pub const ANSI_BRIGHT_WHITE: &str = "#ebdbb2";

pub const FONT_FAMILY: &str = "\"JetBrainsMono Nerd Font Mono\", monospace";
pub const FONT_WEIGHT: i32 = 400;
pub const FONT_SIZE_UI: i32 = 13;
pub const FONT_SIZE_COMPACT: i32 = 11;
pub const LINE_HEIGHT: &str = "1.4";

pub const SPACE_1: i32 = 4;
pub const SPACE_2: i32 = 8;
pub const SPACE_3: i32 = 12;
pub const SPACE_4: i32 = 16;
pub const SPACE_6: i32 = 24;
pub const GAP_MAIN: i32 = SPACE_2;
pub const RADIUS_NONE: i32 = 0;
pub const RADIUS_SUBTLE: i32 = 1;
pub const RADIUS_PANEL: i32 = 2;
pub const BORDER_WIDTH: i32 = 2;
pub const DIVIDER_WIDTH: i32 = 1;

pub const PANEL_WIDTH: i32 = 520;
pub const PANEL_HEIGHT: i32 = 300;
pub const PANEL_MARGIN: i32 = SPACE_4;
pub const HEADER_HEIGHT: i32 = 48;
pub const ICON_SIZE: i32 = 18;
pub const ROW_HEIGHT: i32 = 38;
pub const PANEL_INSET: i32 = 14;
pub const LIST_INSET: i32 = 7;
pub const ICON_NAME_GAP: i32 = 10;
pub const BAR_HEIGHT: i32 = SPACE_4 * 2;
pub const BAR_MARGIN: i32 = SPACE_2;
pub const BAR_CONTENT_PADDING: i32 = 6;
pub const WORKSPACE_SIZE: i32 = SPACE_3 * 2;
pub const WORKSPACE_GAP: i32 = RADIUS_PANEL;
pub const STATUS_GAP: i32 = SPACE_4;
pub const STATUS_ICON_SIZE: i32 = 14;
pub const STATUS_LABEL_GAP: i32 = 5;
pub const CLOCK_DIVIDER_GAP: i32 = 7;
pub const VOLUME_MODULE_WIDTH: i32 = 60;
pub const VOLUME_POPOVER_WIDTH: i32 = 240;
pub const CALENDAR_POPOVER_WIDTH: i32 = VOLUME_POPOVER_WIDTH;
pub const CALENDAR_DAY_SIZE: i32 = WORKSPACE_SIZE + SPACE_1;
pub const VOLUME_OUTPUT_ROW_HEIGHT: i32 = 22;
pub const ICON_STROKE_WIDTH: f64 = 1.25;
pub const BACKDROP_BLUR_PASSES: i32 = 2;
pub const BACKDROP_BLUR_OFFSET: i32 = 2;
pub const BACKDROP_ANIMATION_MS: u32 = 150;

const GTK_TEMPLATE: &str = include_str!("../templates/style.css");
const KBAR_TEMPLATE: &str = include_str!("../templates/kbar.css");
const KBAR_NIRI_TEMPLATE: &str = include_str!("../templates/kbar.kdl");
const NIRI_TEMPLATE: &str = include_str!("../templates/klauncher.kdl");
const MOCKUP_TEMPLATE: &str = include_str!("../templates/theme.css");
const KITTY_TEMPLATE: &str = include_str!("../templates/kitty.conf");
const ALACRITTY_TEMPLATE: &str = include_str!("../templates/alacritty.toml");
const FOOT_TEMPLATE: &str = include_str!("../templates/foot.ini");
const CAVA_TEMPLATE: &str = include_str!("../templates/cava.ini");
const FASTFETCH_TEMPLATE: &str = include_str!("../templates/fastfetch.jsonc");

pub fn render_gtk_css() -> String {
    render(GTK_TEMPLATE)
}

pub fn render_kbar_css() -> String {
    render(KBAR_TEMPLATE)
}

pub fn render_kbar_niri() -> String {
    render(KBAR_NIRI_TEMPLATE)
}

pub fn render_niri_kdl() -> String {
    render(NIRI_TEMPLATE)
}

pub fn render_mockup_css() -> String {
    render(MOCKUP_TEMPLATE)
}

pub fn render_kitty_conf() -> String {
    render(KITTY_TEMPLATE)
}

pub fn render_alacritty_toml() -> String {
    render(ALACRITTY_TEMPLATE)
}

pub fn render_foot_ini() -> String {
    render(FOOT_TEMPLATE)
}

pub fn render_cava_ini() -> String {
    render(CAVA_TEMPLATE)
}

pub fn render_fastfetch_theme() -> String {
    render(FASTFETCH_TEMPLATE)
}

pub fn render_cava_config(config: &str) -> Option<String> {
    const BEGIN: &str = "# BEGIN klauncher-theme";
    const END: &str = "# END klauncher-theme";

    let theme = render_cava_ini();
    if let Some(start) = config.find(BEGIN) {
        let end = config[start..].find(END)? + start + END.len();
        let suffix_start = skip_line_ending(config, end);
        let mut rendered = String::with_capacity(config.len());
        rendered.push_str(&config[..start]);
        rendered.push_str(&theme);
        rendered.push_str(&config[suffix_start..]);
        return Some(rendered);
    }

    let (section_start, section_end) = ini_section_bounds(config, "[color]")?;
    let section = &config[section_start..section_end];
    let mut replacement = theme;
    for line in section.lines() {
        if line.trim_start().starts_with("# END ") {
            replacement.push_str(line);
            replacement.push('\n');
        }
    }

    let mut rendered = String::with_capacity(config.len());
    rendered.push_str(&config[..section_start]);
    rendered.push_str(&replacement);
    rendered.push_str(&config[section_end..]);
    Some(rendered)
}

fn skip_line_ending(input: &str, offset: usize) -> usize {
    let suffix = &input[offset..];
    if suffix.starts_with("\r\n") {
        offset + 2
    } else if suffix.starts_with('\n') {
        offset + 1
    } else {
        offset
    }
}

fn checked_in_generated_files(root: &Path) -> Vec<(PathBuf, String)> {
    vec![
        (
            root.join("apps/klauncher/src/ui/style.css"),
            render_gtk_css(),
        ),
        (root.join("apps/kbar/src/ui/style.css"), render_kbar_css()),
        (root.join("contrib/niri/kbar.kdl"), render_kbar_niri()),
        (root.join("contrib/niri/klauncher.kdl"), render_niri_kdl()),
        (root.join("mockups/theme.css"), render_mockup_css()),
    ]
}

pub fn configured_terminal_files() -> std::io::Result<Vec<(PathBuf, String)>> {
    let mut files = Vec::new();
    let Some(config_home) = config_home() else {
        return Ok(files);
    };

    for theme_path in configured_kitty_themes(&config_home)? {
        files.push((theme_path, render_kitty_conf()));
    }
    if let Some((_config_path, theme_path)) = configured_alacritty_theme(&config_home)? {
        files.push((theme_path, render_alacritty_toml()));
    }
    if let Some(theme_path) = configured_foot_theme(&config_home)? {
        files.push((theme_path, render_foot_ini()));
    }
    if let Some((config_path, rendered)) = configured_cava_config(&config_home)? {
        files.push((config_path, rendered));
    }
    if let Some((config_path, rendered)) = configured_fastfetch_config(&config_home)? {
        files.push((config_path, rendered));
    }

    Ok(files)
}

pub fn generated_files(root: &Path) -> std::io::Result<Vec<(PathBuf, String)>> {
    let mut files = checked_in_generated_files(root);
    for (path, content) in configured_terminal_files()? {
        if content.is_empty() {
            continue;
        }
        files.push((path, content));
    }
    Ok(files)
}

pub fn write_generated_files(root: &Path) -> std::io::Result<()> {
    for (path, content) in checked_in_generated_files(root) {
        std::fs::write(path, content)?;
    }

    let Some(config_home) = config_home() else {
        return Ok(());
    };

    for theme_path in configured_kitty_themes(&config_home)? {
        std::fs::write(theme_path, render_kitty_conf())?;
    }

    if let Some((config_path, theme_path)) = configured_alacritty_theme(&config_home)? {
        std::fs::write(theme_path, render_alacritty_toml())?;
        make_alacritty_opaque(&config_path)?;
    }

    if let Some(theme_path) = configured_foot_theme(&config_home)? {
        std::fs::write(theme_path, render_foot_ini())?;
    }
    if let Some((config_path, rendered)) = configured_cava_config(&config_home)? {
        std::fs::write(config_path, rendered)?;
    }
    if let Some((config_path, rendered)) = configured_fastfetch_config(&config_home)? {
        std::fs::write(config_path, rendered)?;
    }

    Ok(())
}

pub fn generated_files_are_current(root: &Path) -> std::io::Result<bool> {
    for (path, rendered) in generated_files(root)? {
        if std::fs::read_to_string(path)? != rendered {
            return Ok(false);
        }
    }

    let Some(config_home) = config_home() else {
        return Ok(true);
    };
    let Some((config_path, _)) = configured_alacritty_theme(&config_home)? else {
        return Ok(true);
    };

    alacritty_is_opaque(&config_path)
}

fn config_home() -> Option<PathBuf> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        if !config_home.is_empty() {
            let path = PathBuf::from(config_home);
            if path.is_absolute() {
                return Some(path);
            }
        }
    }

    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
}

fn configured_kitty_themes(config_home: &Path) -> std::io::Result<Vec<PathBuf>> {
    if !executable_in_path("kitty") {
        return Ok(Vec::new());
    }

    let config_path = config_home.join("kitty/kitty.conf");
    if !config_path.is_file() {
        return Ok(Vec::new());
    }

    let config = std::fs::read_to_string(&config_path)?;
    let includes = kitty_includes(&config, config_home.join("kitty"));
    Ok(includes
        .into_iter()
        .filter(|path| path.is_file() && is_terminal_theme_path(path))
        .collect())
}

fn configured_alacritty_theme(config_home: &Path) -> std::io::Result<Option<(PathBuf, PathBuf)>> {
    if !executable_in_path("alacritty") {
        return Ok(None);
    }

    let config_path = config_home.join("alacritty/alacritty.toml");
    if !config_path.is_file() {
        return Ok(None);
    }

    let config = std::fs::read_to_string(&config_path)?;
    let theme_path = alacritty_imports(&config, config_path.parent().unwrap_or(config_home))
        .into_iter()
        .find(|path| path.is_file() && is_terminal_theme_path(path));

    Ok(theme_path.map(|theme_path| (config_path, theme_path)))
}

fn configured_foot_theme(config_home: &Path) -> std::io::Result<Option<PathBuf>> {
    if !executable_in_path("foot") {
        return Ok(None);
    }

    let config_path = config_home.join("foot/foot.ini");
    if !config_path.is_file() {
        return Ok(None);
    }

    let config = std::fs::read_to_string(&config_path)?;
    let theme_path = foot_includes(&config, config_path.parent().unwrap_or(config_home))
        .into_iter()
        .rev()
        .find(|path| path.is_file() && is_terminal_theme_path(path));

    Ok(theme_path)
}

fn configured_cava_config(config_home: &Path) -> std::io::Result<Option<(PathBuf, String)>> {
    if !executable_in_path("cava") {
        return Ok(None);
    }

    let config_path = config_home.join("cava/config");
    if !config_path.is_file() {
        return Ok(None);
    }

    let config = std::fs::read_to_string(&config_path)?;
    Ok(render_cava_config(&config).map(|rendered| (config_path, rendered)))
}

fn configured_fastfetch_config(config_home: &Path) -> std::io::Result<Option<(PathBuf, String)>> {
    if !executable_in_path("fastfetch") {
        return Ok(None);
    }

    let config_path = config_home.join("fastfetch/config.jsonc");
    if !config_path.is_file() {
        return Ok(None);
    }

    let config = std::fs::read_to_string(&config_path)?;
    Ok(render_fastfetch_config(&config).map(|rendered| (config_path, rendered)))
}

pub fn render_fastfetch_config(config: &str) -> Option<String> {
    let theme = fastfetch_theme()?;
    let mut replacements = Vec::new();

    if let Some(logo_bounds) = jsonc_object_bounds_after_key(config, "logo", None) {
        if let Some(color_bounds) =
            jsonc_object_bounds_after_key(config, "color", Some(logo_bounds))
        {
            for (index, value) in theme.logo_colors.iter().enumerate() {
                let key = (index + 1).to_string();
                if let Some((start, end)) = jsonc_string_value_span(config, color_bounds, &key) {
                    replacements.push((start, end, value.clone()));
                }
            }
        }
    }

    if let Some(modules_bounds) =
        jsonc_container_bounds_after_key(config, "modules", None, b'[', b']')
    {
        for object_bounds in jsonc_object_ranges(config, modules_bounds) {
            let Some(type_span) = jsonc_string_value_span(config, object_bounds, "type") else {
                continue;
            };
            let Some(key_color_span) = jsonc_string_value_span(config, object_bounds, "keyColor")
            else {
                continue;
            };
            let module_type = &config[type_span.0..type_span.1];
            let value = theme.module_key_color(module_type).to_owned();
            replacements.push((key_color_span.0, key_color_span.1, value));
        }
    }

    if replacements.is_empty() {
        return None;
    }

    Some(replace_jsonc_string_values(config, replacements))
}

struct FastfetchTheme {
    logo_colors: [String; 8],
    module_key_colors: Vec<(&'static str, String)>,
    default_key_color: String,
}

impl FastfetchTheme {
    fn module_key_color(&self, module_type: &str) -> &str {
        self.module_key_colors
            .iter()
            .find(|(name, _)| *name == module_type)
            .map(|(_, value)| value.as_str())
            .unwrap_or(self.default_key_color.as_str())
    }
}

fn fastfetch_theme() -> Option<FastfetchTheme> {
    let rendered = render_fastfetch_theme();
    let logo_values = (1..=8)
        .map(|index| fastfetch_template_value(&rendered, &index.to_string()))
        .collect::<Option<Vec<_>>>()?;
    let logo_colors: [String; 8] = logo_values.try_into().ok()?;

    let mut module_key_colors = Vec::new();
    for module_type in [
        "os", "kernel", "host", "terminal", "packages", "uptime", "cpu", "memory", "disk",
        "battery",
    ] {
        module_key_colors.push((
            module_type,
            fastfetch_template_value(&rendered, module_type)?,
        ));
    }

    Some(FastfetchTheme {
        logo_colors,
        module_key_colors,
        default_key_color: fastfetch_template_value(&rendered, "default")?,
    })
}

fn fastfetch_template_value(template: &str, key: &str) -> Option<String> {
    let span = jsonc_string_value_span(template, (0, template.len()), key)?;
    Some(template[span.0..span.1].to_owned())
}

fn jsonc_object_bounds_after_key(
    input: &str,
    key: &str,
    scope: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    jsonc_container_bounds_after_key(input, key, scope, b'{', b'}')
}

fn jsonc_container_bounds_after_key(
    input: &str,
    key: &str,
    scope: Option<(usize, usize)>,
    opening: u8,
    closing: u8,
) -> Option<(usize, usize)> {
    let (scope_start, scope_end) = scope.unwrap_or((0, input.len()));
    let needle = format!("\"{key}\"");
    let key_start = scope_start + input[scope_start..scope_end].find(&needle)?;
    let colon_start = key_start + needle.len();
    let colon = input[colon_start..scope_end].find(':')? + colon_start;
    let value_start = skip_jsonc_whitespace(input, colon + 1, scope_end);
    if input.as_bytes().get(value_start) != Some(&opening) {
        return None;
    }

    let value_end = matching_jsonc_delimiter(input, value_start, opening, closing)?;
    Some((value_start + 1, value_end))
}

fn jsonc_string_value_span(
    input: &str,
    scope: (usize, usize),
    key: &str,
) -> Option<(usize, usize)> {
    let (scope_start, scope_end) = scope;
    let needle = format!("\"{key}\"");
    let key_start = scope_start + input[scope_start..scope_end].find(&needle)?;
    let colon_start = key_start + needle.len();
    let colon = input[colon_start..scope_end].find(':')? + colon_start;
    let value_quote = skip_jsonc_whitespace(input, colon + 1, scope_end);
    if input.as_bytes().get(value_quote) != Some(&b'\"') {
        return None;
    }

    let value_start = value_quote + 1;
    let bytes = input.as_bytes();
    let mut index = value_start;
    let mut escaped = false;
    while index < scope_end {
        match bytes[index] {
            b'\\' if !escaped => escaped = true,
            b'\"' if !escaped => return Some((value_start, index)),
            _ => escaped = false,
        }
        index += 1;
    }
    None
}

fn skip_jsonc_whitespace(input: &str, mut index: usize, end: usize) -> usize {
    while index < end && input.as_bytes()[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn matching_jsonc_delimiter(
    input: &str,
    opening_index: usize,
    opening: u8,
    closing: u8,
) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut depth = 0;
    let mut index = opening_index;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_line_comment {
            if byte == b'\n' {
                in_line_comment = false;
            }
            index += 1;
            continue;
        }
        if in_block_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if in_string {
            if byte == b'\"' && !escaped {
                in_string = false;
            }
            escaped = byte == b'\\' && !escaped;
            index += 1;
            continue;
        }
        if byte == b'\"' {
            in_string = true;
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            in_line_comment = true;
            index += 2;
            continue;
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_block_comment = true;
            index += 2;
            continue;
        } else if byte == opening {
            depth += 1;
        } else if byte == closing {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn jsonc_object_ranges(input: &str, scope: (usize, usize)) -> Vec<(usize, usize)> {
    let (scope_start, scope_end) = scope;
    let bytes = input.as_bytes();
    let mut ranges = Vec::new();
    let mut index = scope_start;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while index < scope_end {
        let byte = bytes[index];
        if in_line_comment {
            if byte == b'\n' {
                in_line_comment = false;
            }
            index += 1;
            continue;
        }
        if in_block_comment {
            if byte == b'*' && bytes.get(index + 1) == Some(&b'/') {
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if in_string {
            if byte == b'\"' && !escaped {
                in_string = false;
            }
            escaped = byte == b'\\' && !escaped;
            index += 1;
            continue;
        }
        if byte == b'\"' {
            in_string = true;
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            in_line_comment = true;
            index += 2;
            continue;
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            in_block_comment = true;
            index += 2;
            continue;
        } else if byte == b'{' {
            if let Some(end) = matching_jsonc_delimiter(input, index, b'{', b'}') {
                if end <= scope_end {
                    ranges.push((index + 1, end));
                }
            }
        }
        index += 1;
    }
    ranges
}

fn replace_jsonc_string_values(
    input: &str,
    mut replacements: Vec<(usize, usize, String)>,
) -> String {
    replacements.sort_by_key(|replacement| std::cmp::Reverse(replacement.0));
    let mut rendered = input.to_owned();
    for (start, end, value) in replacements {
        rendered.replace_range(start..end, &value);
    }
    rendered
}

fn kitty_includes(config: &str, config_dir: PathBuf) -> Vec<PathBuf> {
    config
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            if fields.next()? != "include" {
                return None;
            }
            expand_config_path(fields.next()?, &config_dir)
        })
        .collect()
}

fn alacritty_imports(config: &str, config_dir: &Path) -> Vec<PathBuf> {
    quoted_values(config)
        .into_iter()
        .filter(|value| value.ends_with(".toml"))
        .filter_map(|value| expand_config_path(&value, config_dir))
        .collect()
}

fn foot_includes(config: &str, config_dir: &Path) -> Vec<PathBuf> {
    config
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            if key.trim() != "include" {
                return None;
            }
            expand_config_path(value.trim(), config_dir)
        })
        .collect()
}

fn ini_section_bounds(config: &str, section: &str) -> Option<(usize, usize)> {
    let mut offset = 0;
    let mut section_start = None;

    for line in config.split_inclusive('\n') {
        if line.trim() == section {
            section_start = Some(offset);
        } else if section_start.is_some() && line.trim().starts_with('[') {
            return Some((section_start?, offset));
        }
        offset += line.len();
    }

    section_start.map(|start| (start, config.len()))
}

fn quoted_values(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remaining = input;
    while let Some(start) = remaining.find('"') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('"') else {
            break;
        };
        values.push(after_start[..end].to_owned());
        remaining = &after_start[end + 1..];
    }
    values
}

fn expand_config_path(value: &str, config_dir: &Path) -> Option<PathBuf> {
    let value = value.trim_matches(['"', '\'']);
    if value.is_empty() {
        return None;
    }

    let home = std::env::var_os("HOME").map(PathBuf::from);
    if value == "$HOME" {
        return home;
    }
    if let Some(suffix) = value.strip_prefix("$HOME/") {
        return home.map(|home| home.join(suffix));
    }
    if let Some(suffix) = value.strip_prefix("~/") {
        return home.map(|home| home.join(suffix));
    }

    let path = Path::new(value);
    Some(if path.is_absolute() {
        path.to_owned()
    } else {
        config_dir.join(path)
    })
}

fn is_terminal_theme_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name.contains("color") || name.contains("theme")
}

fn executable_in_path(binary: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path_var).any(|directory| {
        let candidate = if directory.as_os_str().is_empty() {
            PathBuf::from(binary)
        } else {
            directory.join(binary)
        };
        candidate.is_file() && is_executable_file(&candidate)
    })
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn alacritty_is_opaque(config_path: &Path) -> std::io::Result<bool> {
    let config = std::fs::read_to_string(config_path)?;
    Ok(alacritty_opacity(&config).is_none_or(|opacity| opacity == 1.0))
}

fn alacritty_opacity(config: &str) -> Option<f64> {
    let mut section = "";
    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed;
            continue;
        }
        if section != "[window]" {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() == "opacity" {
            return value
                .split('#')
                .next()
                .and_then(|value| value.trim().parse().ok());
        }
    }
    None
}

fn make_alacritty_opaque(config_path: &Path) -> std::io::Result<()> {
    let config = std::fs::read_to_string(config_path)?;
    if alacritty_is_opaque(config_path)? {
        return Ok(());
    }

    let normalized = normalize_alacritty_opacity(&config);
    std::fs::write(config_path, normalized)
}

fn normalize_alacritty_opacity(config: &str) -> String {
    let mut section = "";
    let mut normalized = String::with_capacity(config.len());

    for line in config.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed;
        }

        if section == "[window]" {
            if let Some((key, _)) = trimmed.split_once('=') {
                if key.trim() == "opacity" {
                    let indent = &line[..line.len() - line.trim_start().len()];
                    normalized.push_str(indent);
                    normalized.push_str("opacity = 1.0");
                    normalized.push('\n');
                    continue;
                }
            }
        }

        normalized.push_str(line);
        normalized.push('\n');
    }

    if !config.ends_with('\n') {
        normalized.pop();
    }
    normalized
}

fn render(template: &str) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{{") {
        rendered.push_str(&remaining[..start]);
        let token_start = start + 2;
        let Some(end) = remaining[token_start..].find("}}") else {
            rendered.push_str(&remaining[start..]);
            return rendered;
        };
        let token_end = token_start + end;
        let name = &remaining[token_start..token_end];
        match token_value(name) {
            Some(value) => rendered.push_str(&value),
            None => rendered.push_str(&remaining[start..token_end + 2]),
        }
        remaining = &remaining[token_end + 2..];
    }

    rendered.push_str(remaining);
    rendered
}

fn token_value(name: &str) -> Option<String> {
    let value = match name {
        "LAUNCHER_NAMESPACE" => kshell_niri::LAUNCHER_NAMESPACE.to_owned(),
        "BAR_NAMESPACE" => kshell_niri::BAR_NAMESPACE.to_owned(),
        "BAR_COMMAND" => kshell_niri::BAR_COMMAND.to_owned(),
        "LAUNCHER_COMMAND" => kshell_niri::LAUNCHER_COMMAND.to_owned(),
        "LAUNCHER_BINDING" => kshell_niri::LAUNCHER_BINDING.to_owned(),
        "BG_CANVAS" => BG_CANVAS.to_owned(),
        "BG_SURFACE" => BG_SURFACE.to_owned(),
        "BG_ELEVATED" => BG_ELEVATED.to_owned(),
        "BG_SELECTED" => BG_SELECTED.to_owned(),
        "BORDER_STRUCTURAL" => BORDER_STRUCTURAL.to_owned(),
        "BORDER_MUTED" => BORDER_MUTED.to_owned(),
        "TEXT_PRIMARY" => TEXT_PRIMARY.to_owned(),
        "TEXT_SECONDARY" => TEXT_SECONDARY.to_owned(),
        "TEXT_DISABLED" => TEXT_DISABLED.to_owned(),
        "BACKDROP_OPACITY" => BACKDROP_OPACITY.to_owned(),
        "ERROR" => ERROR.to_owned(),
        "SUCCESS" => SUCCESS.to_owned(),
        "WARNING" => WARNING.to_owned(),
        "INFORMATION" => INFORMATION.to_owned(),
        "ANSI_BLACK" => ANSI_BLACK.to_owned(),
        "ANSI_RED" => ANSI_RED.to_owned(),
        "ANSI_GREEN" => ANSI_GREEN.to_owned(),
        "ANSI_YELLOW" => ANSI_YELLOW.to_owned(),
        "ANSI_BLUE" => ANSI_BLUE.to_owned(),
        "ANSI_MAGENTA" => ANSI_MAGENTA.to_owned(),
        "ANSI_CYAN" => ANSI_CYAN.to_owned(),
        "ANSI_WHITE" => ANSI_WHITE.to_owned(),
        "ANSI_BRIGHT_BLACK" => ANSI_BRIGHT_BLACK.to_owned(),
        "ANSI_BRIGHT_RED" => ANSI_BRIGHT_RED.to_owned(),
        "ANSI_BRIGHT_GREEN" => ANSI_BRIGHT_GREEN.to_owned(),
        "ANSI_BRIGHT_YELLOW" => ANSI_BRIGHT_YELLOW.to_owned(),
        "ANSI_BRIGHT_BLUE" => ANSI_BRIGHT_BLUE.to_owned(),
        "ANSI_BRIGHT_MAGENTA" => ANSI_BRIGHT_MAGENTA.to_owned(),
        "ANSI_BRIGHT_CYAN" => ANSI_BRIGHT_CYAN.to_owned(),
        "ANSI_BRIGHT_WHITE" => ANSI_BRIGHT_WHITE.to_owned(),
        "FONT_FAMILY" => FONT_FAMILY.to_owned(),
        "FONT_WEIGHT" => FONT_WEIGHT.to_string(),
        "FONT_SIZE_UI" => FONT_SIZE_UI.to_string(),
        "FONT_SIZE_COMPACT" => FONT_SIZE_COMPACT.to_string(),
        "LINE_HEIGHT" => LINE_HEIGHT.to_owned(),
        "SPACE_1" => SPACE_1.to_string(),
        "SPACE_2" => SPACE_2.to_string(),
        "SPACE_3" => SPACE_3.to_string(),
        "SPACE_4" => SPACE_4.to_string(),
        "SPACE_6" => SPACE_6.to_string(),
        "GAP_MAIN" => GAP_MAIN.to_string(),
        "RADIUS_NONE" => RADIUS_NONE.to_string(),
        "RADIUS_SUBTLE" => RADIUS_SUBTLE.to_string(),
        "RADIUS_PANEL" => RADIUS_PANEL.to_string(),
        "BORDER_WIDTH" => BORDER_WIDTH.to_string(),
        "DIVIDER_WIDTH" => DIVIDER_WIDTH.to_string(),
        "PANEL_WIDTH" => PANEL_WIDTH.to_string(),
        "PANEL_HEIGHT" => PANEL_HEIGHT.to_string(),
        "PANEL_MARGIN" => PANEL_MARGIN.to_string(),
        "HEADER_HEIGHT" => HEADER_HEIGHT.to_string(),
        "ICON_SIZE" => ICON_SIZE.to_string(),
        "ROW_HEIGHT" => ROW_HEIGHT.to_string(),
        "PANEL_INSET" => PANEL_INSET.to_string(),
        "LIST_INSET" => LIST_INSET.to_string(),
        "ICON_NAME_GAP" => ICON_NAME_GAP.to_string(),
        "BAR_HEIGHT" => BAR_HEIGHT.to_string(),
        "BAR_CONTENT_PADDING" => BAR_CONTENT_PADDING.to_string(),
        "WORKSPACE_SIZE" => WORKSPACE_SIZE.to_string(),
        "WORKSPACE_GAP" => WORKSPACE_GAP.to_string(),
        "STATUS_GAP" => STATUS_GAP.to_string(),
        "STATUS_ICON_SIZE" => STATUS_ICON_SIZE.to_string(),
        "STATUS_LABEL_GAP" => STATUS_LABEL_GAP.to_string(),
        "CLOCK_DIVIDER_GAP" => CLOCK_DIVIDER_GAP.to_string(),
        "VOLUME_MODULE_WIDTH" => VOLUME_MODULE_WIDTH.to_string(),
        "VOLUME_POPOVER_WIDTH" => VOLUME_POPOVER_WIDTH.to_string(),
        "CALENDAR_POPOVER_WIDTH" => CALENDAR_POPOVER_WIDTH.to_string(),
        "CALENDAR_DAY_SIZE" => CALENDAR_DAY_SIZE.to_string(),
        "VOLUME_OUTPUT_ROW_HEIGHT" => VOLUME_OUTPUT_ROW_HEIGHT.to_string(),
        "ICON_STROKE_WIDTH" => ICON_STROKE_WIDTH.to_string(),
        "BACKDROP_BLUR_PASSES" => BACKDROP_BLUR_PASSES.to_string(),
        "BACKDROP_BLUR_OFFSET" => BACKDROP_BLUR_OFFSET.to_string(),
        _ => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_resolve_all_tokens() {
        for rendered in [
            render_gtk_css(),
            render_kbar_css(),
            render_kbar_niri(),
            render_niri_kdl(),
            render_mockup_css(),
            render_kitty_conf(),
            render_alacritty_toml(),
            render_foot_ini(),
            render_cava_ini(),
            render_fastfetch_theme(),
        ] {
            assert!(
                !rendered.contains("{{"),
                "template contains an unresolved token: {rendered}"
            );
        }
    }

    #[test]
    fn launcher_backdrop_uses_subtle_dim_and_targeted_blur() {
        let gtk_css = render_gtk_css();
        assert!(gtk_css.contains("rgba(0, 0, 0, 0.28)"));

        let niri = render_niri_kdl();
        assert!(niri.contains("match namespace=\"^my-shell-launcher$\""));
        assert!(niri.contains("passes 2"));
        assert!(niri.contains("offset 2"));
        assert!(niri.contains("blur true"));
        assert!(niri.contains("xray false"));
    }

    #[test]
    fn terminal_templates_use_global_surface_tokens_and_complete_ansi_palette() {
        let rendered = [
            render_kitty_conf(),
            render_alacritty_toml(),
            render_foot_ini(),
        ]
        .join("\n");
        for color in [
            BG_CANVAS,
            BG_SELECTED,
            TEXT_PRIMARY,
            ANSI_BLACK,
            ANSI_RED,
            ANSI_GREEN,
            ANSI_YELLOW,
            ANSI_BLUE,
            ANSI_MAGENTA,
            ANSI_CYAN,
            ANSI_WHITE,
            ANSI_BRIGHT_BLACK,
            ANSI_BRIGHT_RED,
            ANSI_BRIGHT_GREEN,
            ANSI_BRIGHT_YELLOW,
            ANSI_BRIGHT_BLUE,
            ANSI_BRIGHT_MAGENTA,
            ANSI_BRIGHT_CYAN,
            ANSI_BRIGHT_WHITE,
        ] {
            assert!(rendered.contains(color), "terminal templates omit {color}");
        }
    }

    #[test]
    fn cava_theme_replaces_only_the_active_color_section() {
        let config = "[general]\nframerate = 60\n\n[color]\nbackground = '#000000'\ngradient = 1\ngradient_color_1 = '#ffffff'\n# END inir-managed\n\n[smoothing]\nnoise_reduction = 77\n";
        let rendered = render_cava_config(config).expect("active Cava color section");

        assert!(rendered.contains("framerate = 60"));
        assert!(rendered.contains("noise_reduction = 77"));
        assert!(rendered.contains("background = '#1d2021'"));
        assert!(rendered.contains("gradient_color_1 = '#3c3836'"));
        assert!(rendered.contains("# END inir-managed"));
        assert!(!rendered.contains("background = '#000000'"));
        assert!(!rendered.contains("gradient_color_1 = '#ffffff'"));
        assert_eq!(rendered.matches("[color]").count(), 1);
        assert_eq!(render_cava_config(&rendered), Some(rendered.clone()));
    }

    #[test]
    fn normalizing_alacritty_opacity_preserves_other_settings() {
        let config =
            "shell = \"fish\"\n\n[window]\nopacity = 0.9\npadding.x = 10\n\n[font]\nsize = 12\n";
        let normalized = normalize_alacritty_opacity(config);

        assert!(normalized.contains("shell = \"fish\""));
        assert!(normalized.contains("opacity = 1.0"));
        assert!(normalized.contains("padding.x = 10"));
        assert!(normalized.contains("size = 12"));
        assert_eq!(normalized.matches("opacity =").count(), 1);
    }

    #[test]
    fn fastfetch_theme_replaces_only_color_values() {
        let config = r##"{
  "logo": {
    "source": "$HOME/.config/fastfetch/logo.txt",
    "color": {
      "1": "black",
      "2": "red",
      "3": "green",
      "4": "yellow",
      "5": "blue",
      "6": "magenta",
      "7": "cyan",
      "8": "white"
    },
    "padding": { "right": 2 }
  },
  "display": { "separator": "  " },
  "modules": [
    { "type": "os", "key": "OS", "keyColor": "31" },
    { "type": "cpu", "key": "CPU", "keyColor": "37", "format": "{}" }
  ]
}"##;
        let rendered = render_fastfetch_config(config).expect("Fastfetch color fields");

        assert!(rendered.contains("\"source\": \"$HOME/.config/fastfetch/logo.txt\""));
        assert!(rendered.contains("\"padding\": { \"right\": 2 }"));
        assert!(rendered.contains("\"1\": \"#ebdbb2\""));
        assert!(rendered.contains("\"7\": \"#ebdbb2\""));
        assert!(rendered.contains("\"type\": \"os\", \"key\": \"OS\", \"keyColor\": \"#458588\""));
        assert!(rendered.contains("\"type\": \"cpu\", \"key\": \"CPU\", \"keyColor\": \"#a89984\""));
        assert!(!rendered.contains("\"keyColor\": \"31\""));
        assert_eq!(render_fastfetch_config(&rendered), Some(rendered.clone()));
    }
}
