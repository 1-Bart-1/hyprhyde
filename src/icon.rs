//! Finding a window's application icon through its `.desktop` file.

use std::fs;
use std::path::{Path, PathBuf};

const FALLBACK_ICON: &str = "application-x-executable";

/// The `applications` directories of the XDG data dirs, most specific first.
pub fn application_dirs() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|dir| !dir.is_empty())
        .unwrap_or_else(|| format!("{home}/.local/share"));
    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    std::iter::once(data_home.as_str())
        .chain(data_dirs.split(':'))
        .map(|dir| Path::new(dir).join("applications"))
        .collect()
}

/// Value of `key` in the `[Desktop Entry]` group of a desktop file's text.
fn desktop_entry_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut in_entry = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
        } else if in_entry {
            let value = line.strip_prefix(key).and_then(|rest| rest.trim_start().strip_prefix('='));
            if let Some(value) = value {
                return Some(value.trim());
            }
        }
    }
    None
}

/// `Icon=` of the desktop file for a window `class`, searched in `dirs`.
///
/// A file whose `StartupWMClass` matches the class wins; otherwise `<class>.desktop`, then its
/// lowercase form.
pub fn desktop_icon(class: &str, dirs: &[PathBuf]) -> Option<String> {
    icon_by_wm_class(class, dirs)
        .or_else(|| icon_by_file_name(class, dirs))
        .filter(|icon| !icon.is_empty())
}

/// `Icon=` of the first desktop file in `dirs` whose `StartupWMClass` is `class`.
fn icon_by_wm_class(class: &str, dirs: &[PathBuf]) -> Option<String> {
    let files =
        dirs.iter().filter_map(|dir| fs::read_dir(dir).ok()).flatten().filter_map(Result::ok);
    files.map(|entry| entry.path()).find_map(|path| {
        if path.extension()? != "desktop" {
            return None;
        }
        let text = fs::read_to_string(&path).ok()?;
        if !desktop_entry_value(&text, "StartupWMClass")?.eq_ignore_ascii_case(class) {
            return None;
        }
        desktop_entry_value(&text, "Icon").map(String::from)
    })
}

/// `Icon=` of `<class>.desktop` or its lowercase form, from the first of `dirs` holding one.
fn icon_by_file_name(class: &str, dirs: &[PathBuf]) -> Option<String> {
    [class.to_string(), class.to_lowercase()].iter().find_map(|stem| {
        let file_name = format!("{stem}.desktop");
        let text = dirs.iter().find_map(|dir| fs::read_to_string(dir.join(&file_name)).ok())?;
        desktop_entry_value(&text, "Icon").map(String::from)
    })
}

/// Image showing the icon of a window `class`, falling back to a generic application icon.
pub fn image_for_class(class: &str) -> gtk::Image {
    let icon = desktop_icon(class, &application_dirs());
    if let Some(path) = icon.as_deref().filter(|icon| Path::new(icon).is_absolute()) {
        return gtk::Image::from_file(path);
    }
    let theme = gtk::IconTheme::for_display(&gtk::gdk::Display::default().expect("a display"));
    let name = icon.filter(|name| theme.has_icon(name)).unwrap_or_else(|| FALLBACK_ICON.into());
    gtk::Image::from_icon_name(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempApplications(PathBuf);

    impl TempApplications {
        fn new(test: &str, files: &[(&str, &str)]) -> Self {
            let dir = std::env::temp_dir().join(format!("hyprhyde-{test}-{}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            for (name, text) in files {
                fs::write(dir.join(name), text).unwrap();
            }
            Self(dir)
        }
    }

    impl Drop for TempApplications {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry(icon: &str, wm_class: Option<&str>) -> String {
        let wm_class =
            wm_class.map(|class| format!("StartupWMClass={class}\n")).unwrap_or_default();
        format!(
            "[Desktop Entry]\nName=App\n{wm_class}Icon={icon}\n[Desktop Action new]\nIcon=wrong\n"
        )
    }

    #[test]
    fn startup_wm_class_match_wins_over_the_file_name() {
        let apps = TempApplications::new(
            "wm-class",
            &[
                ("Code.desktop", &entry("by-name", None)),
                ("vscode.desktop", &entry("by-class", Some("Code"))),
            ],
        );
        assert_eq!(
            desktop_icon("Code", std::slice::from_ref(&apps.0)).as_deref(),
            Some("by-class")
        );
    }

    #[test]
    fn file_name_matches_the_class_or_its_lowercase() {
        let apps = TempApplications::new(
            "file-name",
            &[("kitty.desktop", &entry("kitty", None)), ("Gimp.desktop", &entry("gimp", None))],
        );
        let dirs = [apps.0.clone()];
        assert_eq!(desktop_icon("kitty", &dirs).as_deref(), Some("kitty"));
        assert_eq!(desktop_icon("Kitty", &dirs).as_deref(), Some("kitty"));
        assert_eq!(desktop_icon("Gimp", &dirs).as_deref(), Some("gimp"));
    }

    #[test]
    fn earlier_dirs_shadow_later_ones() {
        let user = TempApplications::new("user", &[("kitty.desktop", &entry("user-kitty", None))]);
        let system =
            TempApplications::new("system", &[("kitty.desktop", &entry("system-kitty", None))]);
        let dirs = [user.0.clone(), system.0.clone()];
        assert_eq!(desktop_icon("kitty", &dirs).as_deref(), Some("user-kitty"));
    }

    #[test]
    fn unknown_class_or_missing_icon_finds_nothing() {
        let apps =
            TempApplications::new("missing", &[("bare.desktop", "[Desktop Entry]\nName=Bare\n")]);
        let dirs = [apps.0.clone(), PathBuf::from("/nonexistent")];
        assert_eq!(desktop_icon("bare", &dirs), None);
        assert_eq!(desktop_icon("nothing", &dirs), None);
    }

    #[test]
    fn only_the_desktop_entry_group_counts() {
        let text = "[Desktop Action new]\nIcon=wrong\n[Desktop Entry]\nIcon = right\n";
        assert_eq!(desktop_entry_value(text, "Icon"), Some("right"));
        assert_eq!(desktop_entry_value("[Desktop Entry]\nIconName=x\n", "Icon"), None);
    }
}
