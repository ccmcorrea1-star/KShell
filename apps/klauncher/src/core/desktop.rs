use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub generic_name: Option<String>,
    pub icon: Option<String>,
    pub exec: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub terminal: bool,
}

pub fn load_applications() -> Vec<DesktopEntry> {
    let mut applications = Vec::new();
    let mut seen_files = HashSet::new();

    for directory in application_directories() {
        let mut files = Vec::new();
        collect_desktop_files(&directory, &mut files);

        for path in files {
            let Some(file_id) = desktop_file_id(&directory, &path) else {
                continue;
            };

            if !seen_files.insert(file_id) {
                continue;
            }

            if let Ok(Some(application)) = parse_desktop_file(&path) {
                applications.push(application);
            }
        }
    }

    applications.sort_by_cached_key(|application| application.name.to_lowercase());
    applications
}

fn collect_desktop_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            collect_desktop_files(&path, files);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("desktop")
            && fs::metadata(&path).is_ok_and(|metadata| metadata.is_file())
        {
            files.push(path);
        }
    }
}

fn desktop_file_id(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut components = relative.components();
    let first = components.next()?.as_os_str().to_str()?.to_owned();
    let mut id = first;
    for component in components {
        id.push('-');
        id.push_str(component.as_os_str().to_str()?);
    }
    Some(id)
}

fn application_directories() -> Vec<PathBuf> {
    let mut directories = Vec::with_capacity(4);

    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));

    if let Some(data_home) = data_home {
        directories.push(data_home.join("applications"));
    }

    let data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|directories| env::split_paths(&directories).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|path| path.is_absolute())
        .collect::<Vec<_>>();

    if data_dirs.is_empty() {
        directories.push(PathBuf::from("/usr/local/share/applications"));
        directories.push(PathBuf::from("/usr/share/applications"));
    } else {
        directories.extend(
            data_dirs
                .into_iter()
                .map(|directory| directory.join("applications")),
        );
    }

    directories
}

fn parse_desktop_file(path: &Path) -> io::Result<Option<DesktopEntry>> {
    let contents = read_desktop_file(path)?;
    Ok(parse_desktop_entry(&contents, path))
}

#[cfg(unix)]
fn read_desktop_file(path: &Path) -> io::Result<String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    let mut file = options.open(path)?;

    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "desktop entry is not a regular file",
        ));
    }

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

#[cfg(not(unix))]
fn read_desktop_file(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

fn parse_desktop_entry(contents: &str, path: &Path) -> Option<DesktopEntry> {
    let mut in_desktop_entry_group = false;
    let mut values = Vec::new();

    for line in contents.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(group) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            in_desktop_entry_group = group == "Desktop Entry";
            continue;
        }

        if !in_desktop_entry_group || line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.push((key.trim(), value.trim()));
    }

    let get = |key: &str| {
        values
            .iter()
            .rev()
            .find_map(|(current_key, value)| (*current_key == key).then_some(*value))
    };

    if get("Type") != Some("Application")
        || get("Hidden") == Some("true")
        || get("NoDisplay") == Some("true")
        || !is_visible_in_current_desktop(get("OnlyShowIn"), get("NotShowIn"))
        || !try_exec_is_available(get("TryExec"))
    {
        return None;
    }

    let name = localized_value(&values, "Name", preferred_locales())?;
    let raw_exec = get("Exec")?;
    let icon = get("Icon").map(unescape_value);
    let exec = parse_exec(raw_exec, &name, path, icon.as_deref()).ok()?;

    if exec.is_empty() {
        return None;
    }

    Some(DesktopEntry {
        name,
        generic_name: localized_value(&values, "GenericName", preferred_locales()),
        icon,
        exec,
        working_dir: get("Path")
            .map(unescape_value)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
        terminal: get("Terminal") == Some("true"),
    })
}

fn is_visible_in_current_desktop(only_show_in: Option<&str>, not_show_in: Option<&str>) -> bool {
    let current_desktops = env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .map(|value| {
            value
                .split(':')
                .filter(|desktop| !desktop.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    is_visible_in_desktops(
        &current_desktops
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        only_show_in,
        not_show_in,
    )
}

fn is_visible_in_desktops(
    current_desktops: &[&str],
    only_show_in: Option<&str>,
    not_show_in: Option<&str>,
) -> bool {
    for current in current_desktops
        .iter()
        .copied()
        .filter(|desktop| !desktop.is_empty())
    {
        if only_show_in
            .is_some_and(|desktops| desktops.split(';').any(|desktop| desktop == current))
        {
            return true;
        }
        if not_show_in.is_some_and(|desktops| desktops.split(';').any(|desktop| desktop == current))
        {
            return false;
        }
    }

    only_show_in.is_none()
}

fn try_exec_is_available(value: Option<&str>) -> bool {
    let Some(value) = value.map(unescape_value).filter(|value| !value.is_empty()) else {
        return true;
    };

    let candidates = if Path::new(&value).is_absolute() {
        vec![PathBuf::from(value)]
    } else {
        env::var_os("PATH")
            .map(|path| {
                env::split_paths(&path)
                    .map(|directory| directory.join(&value))
                    .collect()
            })
            .unwrap_or_default()
    };

    candidates.into_iter().any(|path| is_executable_file(&path))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn preferred_locales() -> Vec<String> {
    let lc_all = env::var_os("LC_ALL").map(|locale| locale.to_string_lossy().into_owned());
    let lc_messages =
        env::var_os("LC_MESSAGES").map(|locale| locale.to_string_lossy().into_owned());
    let lang = env::var_os("LANG").map(|locale| locale.to_string_lossy().into_owned());

    preferred_locales_for(lc_all.as_deref(), lc_messages.as_deref(), lang.as_deref())
}

fn preferred_locales_for(
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    lang: Option<&str>,
) -> Vec<String> {
    let Some(locale) = [lc_all, lc_messages, lang]
        .into_iter()
        .flatten()
        .find(|locale| !locale.is_empty())
    else {
        return Vec::new();
    };

    locale_candidates(locale)
}

fn locale_candidates(locale: &str) -> Vec<String> {
    let (without_modifier, modifier) = locale
        .split_once('@')
        .map_or((locale, None), |(locale, modifier)| {
            (locale, Some(modifier))
        });
    let without_encoding = without_modifier
        .split_once('.')
        .map_or(without_modifier, |(locale, _)| locale);
    let (language, country) = without_encoding
        .split_once('_')
        .map_or((without_encoding, None), |(language, country)| {
            (language, Some(country))
        });
    let mut candidates = Vec::with_capacity(4);

    if let (Some(country), Some(modifier)) = (country, modifier) {
        candidates.push(format!("{language}_{country}@{modifier}"));
    }
    if let Some(country) = country {
        candidates.push(format!("{language}_{country}"));
    }
    if let Some(modifier) = modifier {
        candidates.push(format!("{language}@{modifier}"));
    }
    if !language.is_empty() {
        candidates.push(language.to_owned());
    }

    candidates
}

fn localized_value(values: &[(&str, &str)], key: &str, locales: Vec<String>) -> Option<String> {
    for locale in locales {
        let localized_key = format!("{key}[{locale}]");
        if let Some(value) = values
            .iter()
            .rev()
            .find_map(|(current_key, value)| (*current_key == localized_key).then_some(*value))
        {
            let value = unescape_value(value);
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    values
        .iter()
        .rev()
        .find_map(|(current_key, value)| (*current_key == key).then_some(unescape_value(value)))
        .filter(|value| !value.is_empty())
}

fn unescape_value(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            unescaped.push(match character {
                's' => ' ',
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            unescaped.push(character);
        }
    }

    if escaped {
        unescaped.push('\\');
    }

    unescaped
}

fn parse_exec(
    command_line: &str,
    name: &str,
    desktop_file: &Path,
    icon: Option<&str>,
) -> Result<Vec<String>, ()> {
    let command_line = unescape_exec_value(command_line);
    let words = split_exec_words(&command_line)?;
    if words
        .iter()
        .map(|word| count_file_field_codes(&word.value))
        .sum::<usize>()
        > 1
    {
        return Err(());
    }
    let mut expanded = Vec::with_capacity(words.len());

    for word in words {
        if word.quoted_field_code && contains_field_code(&word.value) {
            return Err(());
        }

        if word.value == "%i" {
            if let Some(icon) = icon.filter(|icon| !icon.is_empty()) {
                expanded.push("--icon".to_owned());
                expanded.push(icon.to_owned());
            }
            continue;
        }

        if contains_standalone_only_field_code(&word.value) && word.value.len() != 2 {
            return Err(());
        }

        let mut value = String::with_capacity(word.value.len());
        let mut characters = word.value.chars();
        while let Some(character) = characters.next() {
            if character != '%' {
                value.push(character);
                continue;
            }

            let code = characters.next().ok_or(())?;
            match code {
                '%' => value.push('%'),
                'c' => value.push_str(name),
                'k' => value.push_str(&desktop_file.to_string_lossy()),
                'f' | 'F' | 'u' | 'U' | 'd' | 'D' | 'n' | 'N' | 'v' | 'm' => {}
                _ => return Err(()),
            }
        }

        if !value.is_empty() || word.quoted {
            expanded.push(value);
        }
    }

    Ok(expanded)
}

fn unescape_exec_value(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            unescaped.push(character);
            continue;
        }

        match characters.next() {
            Some('s') => unescaped.push(' '),
            Some('n') => unescaped.push('\n'),
            Some('t') => unescaped.push('\t'),
            Some('r') => unescaped.push('\r'),
            Some('\\') => unescaped.push('\\'),
            Some(other) => {
                unescaped.push('\\');
                unescaped.push(other);
            }
            None => unescaped.push('\\'),
        }
    }

    unescaped
}

struct ExecWord {
    value: String,
    quoted: bool,
    quoted_field_code: bool,
}

fn split_exec_words(command_line: &str) -> Result<Vec<ExecWord>, ()> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;
    let mut has_content = false;
    let mut quoted = false;
    let mut quoted_field_code = false;

    for character in command_line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            has_content = true;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '%' if in_quotes => {
                current.push(character);
                quoted_field_code = true;
                has_content = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                quoted = true;
                has_content = true;
            }
            character if character.is_whitespace() && !in_quotes => {
                if has_content {
                    words.push(ExecWord {
                        value: std::mem::take(&mut current),
                        quoted,
                        quoted_field_code,
                    });
                    has_content = false;
                    quoted = false;
                    quoted_field_code = false;
                }
            }
            character => {
                current.push(character);
                has_content = true;
            }
        }
    }

    if escaped || in_quotes {
        return Err(());
    }

    if has_content {
        words.push(ExecWord {
            value: current,
            quoted,
            quoted_field_code,
        });
    }

    Ok(words)
}

fn contains_field_code(value: &str) -> bool {
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '%' {
            continue;
        }

        match characters.next() {
            Some('%') => {}
            Some(code) if code.is_ascii_alphabetic() => return true,
            Some(_) | None => {}
        }
    }

    false
}

fn count_file_field_codes(value: &str) -> usize {
    let mut count = 0;
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '%' {
            continue;
        }

        match characters.next() {
            Some('%') => {}
            Some('f' | 'F' | 'u' | 'U') => count += 1,
            Some(_) | None => {}
        }
    }

    count
}

fn contains_standalone_only_field_code(value: &str) -> bool {
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '%' {
            continue;
        }

        match characters.next() {
            Some('%') => {}
            Some('F' | 'U' | 'i') => return true,
            Some(_) | None => {}
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{
        is_visible_in_desktops, locale_candidates, parse_desktop_entry, parse_exec,
        preferred_locales_for,
    };
    use std::path::Path;

    #[test]
    fn parses_application_and_expands_exec_fields() {
        let entry = parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=Example\nIcon=example\nExec=example --title %c %i %F %%\n",
            Path::new("/tmp/example.desktop"),
        )
        .expect("valid application");

        assert_eq!(entry.name, "Example");
        assert_eq!(entry.icon.as_deref(), Some("example"));
        assert_eq!(
            entry.exec,
            vec!["example", "--title", "Example", "--icon", "example", "%",]
        );
    }

    #[test]
    fn ignores_hidden_nodisplay_and_non_application_entries() {
        for metadata in ["Hidden=true\n", "NoDisplay=true\n", "Type=Link\n"] {
            let contents = format!(
                "[Desktop Entry]\nType=Application\nName=Example\nExec=example\n{metadata}"
            );
            assert!(parse_desktop_entry(&contents, Path::new("example.desktop")).is_none());
        }
    }

    #[test]
    fn uses_exec_as_fallback_for_dbus_activatable_entries() {
        let contents =
            "[Desktop Entry]\nType=Application\nName=Example\nExec=example\nDBusActivatable=true\n";
        let entry = parse_desktop_entry(contents, Path::new("example.desktop"))
            .expect("Exec remains a compatibility fallback");

        assert_eq!(entry.exec, vec!["example"]);
        assert!(parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=Example\nDBusActivatable=true\n",
            Path::new("example.desktop"),
        )
        .is_none());
    }

    #[test]
    fn removes_file_placeholders_without_using_a_shell() {
        let command = parse_exec(
            "browser --new-window %U --name \"A quoted value\"",
            "Browser",
            Path::new("browser.desktop"),
            None,
        )
        .expect("valid exec line");

        assert_eq!(
            command,
            vec!["browser", "--new-window", "--name", "A quoted value"]
        );
    }

    #[test]
    fn applies_general_exec_escapes_before_quoting() {
        let command = parse_exec(
            r#"app "a\sb" "a\\\\b""#,
            "App",
            Path::new("app.desktop"),
            None,
        )
        .expect("valid exec line");

        assert_eq!(command, vec!["app", "a b", r"a\b"]);
    }

    #[test]
    fn preserves_empty_quoted_exec_arguments() {
        let command = parse_exec(r#"app "" """#, "App", Path::new("app.desktop"), None)
            .expect("valid exec line");

        assert_eq!(command, vec!["app", "", ""]);
    }

    #[test]
    fn treats_escaped_percent_sequences_as_literal_percentages() {
        let command = parse_exec(
            r#"app "100%%c" "%%f""#,
            "App",
            Path::new("app.desktop"),
            None,
        )
        .expect("valid exec line");

        assert_eq!(command, vec!["app", "100%c", "%f"]);
    }

    #[test]
    fn rejects_multiple_file_field_codes() {
        assert!(parse_exec("app %f %u", "App", Path::new("app.desktop"), None,).is_err());
        assert!(parse_exec("app %F %U", "App", Path::new("app.desktop"), None,).is_err());
    }

    #[test]
    fn resolves_desktop_visibility_in_current_desktop_order() {
        assert!(!is_visible_in_desktops(
            &["GNOME", "KDE"],
            Some("KDE"),
            Some("GNOME"),
        ));
        assert!(is_visible_in_desktops(
            &["KDE", "GNOME"],
            Some("KDE"),
            Some("GNOME"),
        ));
    }

    #[test]
    fn generates_locale_fallbacks_for_country_and_modifier() {
        assert_eq!(
            locale_candidates("sr_YU.UTF-8@Latn"),
            vec!["sr_YU@Latn", "sr_YU", "sr@Latn", "sr"]
        );
        assert_eq!(locale_candidates("sr_YU.UTF-8"), vec!["sr_YU", "sr"]);
        assert_eq!(locale_candidates("sr.UTF-8@Latn"), vec!["sr@Latn", "sr"]);
    }

    #[test]
    fn selects_effective_posix_locale_in_precedence_order() {
        assert_eq!(
            preferred_locales_for(Some("de_DE.UTF-8"), Some("fr_FR"), Some("en_US")),
            vec!["de_DE", "de"]
        );
        assert_eq!(
            preferred_locales_for(None, Some("fr_FR"), Some("en_US")),
            vec!["fr_FR", "fr"]
        );
        assert_eq!(
            preferred_locales_for(None, None, Some("en_US")),
            vec!["en_US", "en"]
        );
        assert_eq!(
            preferred_locales_for(Some(""), Some("fr_FR"), Some("en_US")),
            vec!["fr_FR", "fr"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn collects_only_regular_desktop_files() {
        use std::ffi::CString;
        use std::fs;
        use std::os::unix::ffi::OsStrExt;
        use std::time::{SystemTime, UNIX_EPOCH};

        let directory = std::env::temp_dir().join(format!(
            "klauncher-desktop-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock after epoch")
                .as_nanos()
        ));
        fs::create_dir(&directory).expect("create test directory");
        let regular = directory.join("regular.desktop");
        let fifo = directory.join("special.desktop");
        fs::write(&regular, "[Desktop Entry]\n").expect("create regular desktop file");
        let fifo_path = CString::new(fifo.as_os_str().as_bytes()).expect("valid fifo path");
        assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);

        let mut files = Vec::new();
        super::collect_desktop_files(&directory, &mut files);
        assert!(super::parse_desktop_file(&fifo).is_err());
        fs::remove_dir_all(&directory).expect("remove test directory");

        assert_eq!(files, vec![regular]);
    }

    #[test]
    fn trims_entry_keys_and_reads_terminal_metadata() {
        let entry = parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName = Example\nExec = example\nTerminal=true\n",
            Path::new("example.desktop"),
        )
        .expect("valid application");

        assert_eq!(entry.name, "Example");
        assert!(entry.terminal);
    }

    #[test]
    fn rejects_field_codes_inside_quoted_arguments() {
        assert!(parse_exec(
            "browser \"%i\"",
            "Browser",
            Path::new("browser.desktop"),
            Some("browser"),
        )
        .is_err());
        assert!(parse_exec(
            "browser --files%F",
            "Browser",
            Path::new("browser.desktop"),
            None,
        )
        .is_err());
    }

    #[test]
    fn builds_desktop_file_ids_from_relative_paths() {
        assert_eq!(
            super::desktop_file_id(
                Path::new("/usr/share/applications"),
                Path::new("/usr/share/applications/foo/bar.desktop"),
            ),
            Some("foo-bar.desktop".to_owned())
        );
    }
}
