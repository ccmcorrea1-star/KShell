use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub generic_name: Option<String>,
    pub exec: Vec<String>,
    pub working_dir: Option<PathBuf>,
}

pub fn load_applications() -> Vec<DesktopEntry> {
    let mut applications = Vec::new();
    let mut seen_files = HashSet::new();

    for directory in application_directories() {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("desktop") {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if !seen_files.insert(file_name.to_owned()) {
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
    let contents = fs::read_to_string(path)?;
    Ok(parse_desktop_entry(&contents, path))
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
        values.push((key, value.trim()));
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
        exec,
        working_dir: get("Path")
            .map(unescape_value)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from),
    })
}

fn preferred_locales() -> Vec<String> {
    let mut locales = Vec::new();

    if let Some(language) = env::var_os("LANGUAGE") {
        locales.extend(
            language
                .to_string_lossy()
                .split(':')
                .filter(|locale| !locale.is_empty())
                .map(str::to_owned),
        );
    }

    for variable in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(locale) = env::var_os(variable) {
            let locale = locale.to_string_lossy();
            if !locale.is_empty() {
                locales.push(locale.into_owned());
            }
        }
    }

    let mut expanded = Vec::with_capacity(locales.len() * 3);
    for locale in locales {
        let without_encoding = locale.split('.').next().unwrap_or(&locale);
        let without_modifier = without_encoding
            .split('@')
            .next()
            .unwrap_or(without_encoding);

        for candidate in [locale.as_str(), without_encoding, without_modifier] {
            if !candidate.is_empty() && !expanded.iter().any(|item| item == candidate) {
                expanded.push(candidate.to_owned());
            }
        }
    }

    expanded
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
    let words = split_exec_words(command_line)?;
    let mut expanded = Vec::with_capacity(words.len());

    for word in words {
        if word == "%i" {
            if let Some(icon) = icon.filter(|icon| !icon.is_empty()) {
                expanded.push("--icon".to_owned());
                expanded.push(icon.to_owned());
            }
            continue;
        }

        let mut value = String::with_capacity(word.len());
        let mut characters = word.chars();
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

        if !value.is_empty() {
            expanded.push(value);
        }
    }

    Ok(expanded)
}

fn split_exec_words(command_line: &str) -> Result<Vec<String>, ()> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;
    let mut has_content = false;

    for character in command_line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            has_content = true;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => {
                in_quotes = !in_quotes;
                has_content = true;
            }
            character if character.is_whitespace() && !in_quotes => {
                if has_content {
                    words.push(std::mem::take(&mut current));
                    has_content = false;
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
        words.push(current);
    }

    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::{parse_desktop_entry, parse_exec};
    use std::path::Path;

    #[test]
    fn parses_application_and_expands_exec_fields() {
        let entry = parse_desktop_entry(
            "[Desktop Entry]\nType=Application\nName=Example\nIcon=example\nExec=example --title %c %i %F %%\n",
            Path::new("/tmp/example.desktop"),
        )
        .expect("valid application");

        assert_eq!(entry.name, "Example");
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
}
