use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey, HKEY,
};

use crate::{
    errors::{AppError, AppResult},
    models::AppConfig,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchTarget {
    path: PathBuf,
    arguments: Vec<String>,
    use_shell: bool,
}

pub fn launch_app_with_path(
    app: &AppConfig,
    executable_path: Option<&str>,
) -> AppResult<Option<u32>> {
    if app.launch_delay_seconds > 0 {
        thread::sleep(Duration::from_secs(app.launch_delay_seconds));
    }

    let target = resolve_launch_target(app, executable_path).ok_or_else(|| {
        AppError::InvalidExecutablePath(format!(
            "Could not find a launch target for {}",
            app.display_name
        ))
    })?;

    if target.use_shell {
        Command::new("explorer").arg(&target.path).spawn()?;
        return Ok(None);
    }

    let mut command = Command::new(&target.path);
    command.args(&target.arguments);
    command.args(&app.arguments);

    if let Some(working_directory) = &app.working_directory {
        let working_directory = working_directory.trim();
        if !working_directory.is_empty() && Path::new(working_directory).exists() {
            command.current_dir(working_directory);
        }
    } else if let Some(parent) = target.path.parent() {
        command.current_dir(parent);
    }

    let child = command.spawn()?;
    Ok(Some(child.id()))
}

fn resolve_launch_target(app: &AppConfig, preferred_path: Option<&str>) -> Option<LaunchTarget> {
    launch_candidates(app, preferred_path)
        .into_iter()
        .find_map(resolve_candidate)
}

fn launch_candidates(app: &AppConfig, preferred_path: Option<&str>) -> Vec<LaunchTarget> {
    let mut candidates = Vec::new();

    push_path_candidate(&mut candidates, preferred_path);
    push_path_candidate(&mut candidates, app.executable_path.as_deref());

    if let Some(process_name) = configured_process_name(app) {
        candidates.extend(app_path_registry_candidates(&process_name));
        candidates.extend(uninstall_registry_candidates(app, &process_name));
        candidates.extend(known_install_candidates(app, &process_name));
        candidates.extend(start_menu_candidates(app, &process_name));
    }

    dedupe_candidates(candidates)
}

fn push_path_candidate(candidates: &mut Vec<LaunchTarget>, path: Option<&str>) {
    let Some(path) = path.map(str::trim).filter(|path| !path.is_empty()) else {
        return;
    };

    let path = expand_environment(path);
    let path = path.trim().trim_matches('"');
    if path.is_empty() {
        return;
    }

    candidates.push(LaunchTarget {
        path: PathBuf::from(path),
        arguments: Vec::new(),
        use_shell: is_shell_item(path),
    });
}

fn resolve_candidate(candidate: LaunchTarget) -> Option<LaunchTarget> {
    if candidate.path.exists() {
        return Some(candidate);
    }

    if has_path_separator(&candidate.path) {
        return None;
    }

    find_on_path(&candidate.path).map(|path| LaunchTarget {
        path,
        arguments: candidate.arguments,
        use_shell: candidate.use_shell,
    })
}

fn configured_process_name(app: &AppConfig) -> Option<String> {
    app.process_name
        .as_ref()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            app.executable_path
                .as_ref()
                .and_then(|path| Path::new(path).file_name())
                .map(|name| name.to_string_lossy().to_string())
        })
}

fn app_path_registry_candidates(process_name: &str) -> Vec<LaunchTarget> {
    app_path_names(process_name)
        .into_iter()
        .flat_map(|name| {
            [
                registry_string(
                    HKEY_CURRENT_USER,
                    &format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{name}"),
                    "",
                ),
                registry_string(
                    HKEY_LOCAL_MACHINE,
                    &format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{name}"),
                    "",
                ),
            ]
        })
        .flatten()
        .map(|path| LaunchTarget {
            path: PathBuf::from(expand_environment(&path)),
            arguments: Vec::new(),
            use_shell: false,
        })
        .collect()
}

fn app_path_names(process_name: &str) -> Vec<String> {
    let mut names = vec![process_name.to_string()];
    if !process_name.to_ascii_lowercase().ends_with(".exe") {
        names.push(format!("{process_name}.exe"));
    }
    names
}

fn uninstall_registry_candidates(app: &AppConfig, process_name: &str) -> Vec<LaunchTarget> {
    let roots = [
        (
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
    ];
    let mut candidates = Vec::new();

    for (hive, root) in roots {
        let Ok(key) = RegKey::predef(hive).open_subkey(root) else {
            continue;
        };

        for subkey_name in key.enum_keys().flatten() {
            let Ok(subkey) = key.open_subkey(subkey_name) else {
                continue;
            };
            let Ok(display_name) = subkey.get_value::<String, _>("DisplayName") else {
                continue;
            };
            if !installed_name_matches(&display_name, app, process_name) {
                continue;
            }

            if let Ok(display_icon) = subkey.get_value::<String, _>("DisplayIcon") {
                push_path_candidate(
                    &mut candidates,
                    Some(clean_registry_path(&display_icon).as_str()),
                );
            }
            if let Ok(install_location) = subkey.get_value::<String, _>("InstallLocation") {
                let install_location = clean_registry_path(&install_location);
                push_path_candidate(
                    &mut candidates,
                    Some(
                        Path::new(&install_location)
                            .join(process_name)
                            .to_string_lossy()
                            .as_ref(),
                    ),
                );
            }
        }
    }

    candidates
}

fn installed_name_matches(display_name: &str, app: &AppConfig, process_name: &str) -> bool {
    let display_name = normalize_name(display_name);
    let app_name = normalize_name(&app.display_name);
    let process_stem = normalize_name(file_stem(process_name).as_deref().unwrap_or(process_name));

    display_name == app_name || (!process_stem.is_empty() && display_name.contains(&process_stem))
}

fn known_install_candidates(app: &AppConfig, process_name: &str) -> Vec<LaunchTarget> {
    let mut candidates = Vec::new();
    let process_stem = file_stem(process_name).unwrap_or_else(|| process_name.to_string());
    let display_name = app.display_name.trim();
    let compact_display = display_name.replace(' ', "");
    let folder_names = unique_strings([
        display_name.to_string(),
        compact_display,
        process_stem.clone(),
    ]);

    for root in common_install_roots() {
        for folder in &folder_names {
            push_path_candidate(
                &mut candidates,
                Some(
                    root.join(folder)
                        .join(process_name)
                        .to_string_lossy()
                        .as_ref(),
                ),
            );
            push_path_candidate(
                &mut candidates,
                Some(
                    root.join("Programs")
                        .join(folder)
                        .join(process_name)
                        .to_string_lossy()
                        .as_ref(),
                ),
            );
            candidates.extend(squirrel_app_candidates(&root.join(folder), process_name));
        }
    }

    let normalized = normalize_name(process_name);
    if is_github_desktop_alias(&normalized) {
        if let Some(local_app_data) = env_path("LOCALAPPDATA") {
            push_path_candidate(
                &mut candidates,
                Some(
                    local_app_data
                        .join("GitHubDesktop")
                        .join("GitHubDesktop.exe")
                        .to_string_lossy()
                        .as_ref(),
                ),
            );
            candidates.extend(squirrel_app_candidates(
                &local_app_data.join("GitHubDesktop"),
                "GitHubDesktop.exe",
            ));
            push_path_candidate(
                &mut candidates,
                Some(
                    local_app_data
                        .join("Programs")
                        .join("GitHub Desktop")
                        .join("GitHubDesktop.exe")
                        .to_string_lossy()
                        .as_ref(),
                ),
            );
        }
    }

    if normalized == "discord" || normalized == "discordexe" {
        if let Some(local_app_data) = env_path("LOCALAPPDATA") {
            candidates.extend(squirrel_app_candidates(
                &local_app_data.join("Discord"),
                "Discord.exe",
            ));
        }
    }

    if normalized == "obs64" || normalized == "obs64exe" {
        if let Some(program_files) = env_path("ProgramFiles") {
            push_path_candidate(
                &mut candidates,
                Some(
                    program_files
                        .join("obs-studio")
                        .join("bin")
                        .join("64bit")
                        .join("obs64.exe")
                        .to_string_lossy()
                        .as_ref(),
                ),
            );
        }
    }

    candidates
}

fn is_github_desktop_alias(normalized_process_name: &str) -> bool {
    matches!(
        normalized_process_name,
        "github" | "githubexe" | "githubdesktop" | "githubdesktopexe"
    )
}

fn squirrel_app_candidates(app_root: &Path, process_name: &str) -> Vec<LaunchTarget> {
    let Ok(entries) = fs::read_dir(app_root) else {
        return Vec::new();
    };

    let mut candidates = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|item| item.is_dir()).unwrap_or(false))
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .starts_with("app-")
        })
        .filter_map(|entry| {
            let path = entry.path().join(process_name);
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok();
            if path.exists() {
                Some((
                    modified,
                    LaunchTarget {
                        path,
                        arguments: Vec::new(),
                        use_shell: false,
                    },
                ))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    candidates
        .into_iter()
        .map(|(_, candidate)| candidate)
        .collect()
}

fn start_menu_candidates(app: &AppConfig, process_name: &str) -> Vec<LaunchTarget> {
    let mut candidates = Vec::new();
    let targets = unique_strings([
        normalize_name(&app.display_name),
        normalize_name(file_stem(process_name).as_deref().unwrap_or(process_name)),
    ]);

    for root in start_menu_roots() {
        collect_shortcuts(&root, &targets, &mut candidates);
    }

    candidates
}

fn collect_shortcuts(root: &Path, targets: &[String], candidates: &mut Vec<LaunchTarget>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().map(|item| item.is_dir()).unwrap_or(false) {
            collect_shortcuts(&path, targets, candidates);
            continue;
        }

        let Some(ext) = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        else {
            continue;
        };
        if ext != "lnk" && ext != "url" {
            continue;
        }

        let Some(stem) = path
            .file_stem()
            .map(|name| normalize_name(&name.to_string_lossy()))
        else {
            continue;
        };
        if !targets
            .iter()
            .any(|target| stem == *target || (!target.is_empty() && stem.contains(target)))
        {
            continue;
        }

        candidates.push(LaunchTarget {
            path,
            arguments: Vec::new(),
            use_shell: true,
        });
    }
}

fn common_install_roots() -> Vec<PathBuf> {
    [
        "LOCALAPPDATA",
        "APPDATA",
        "ProgramFiles",
        "ProgramFiles(x86)",
    ]
    .into_iter()
    .filter_map(env_path)
    .collect()
}

fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(app_data) = env_path("APPDATA") {
        roots.push(
            app_data
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(program_data) = env_path("ProgramData") {
        roots.push(
            program_data
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    roots
}

fn registry_string(hive: HKEY, subkey: &str, value: &str) -> Option<String> {
    RegKey::predef(hive)
        .open_subkey(subkey)
        .ok()?
        .get_value(value)
        .ok()
}

fn clean_registry_path(path: &str) -> String {
    path.split(',')
        .next()
        .unwrap_or(path)
        .trim()
        .trim_matches('"')
        .to_string()
}

fn expand_environment(path: &str) -> String {
    let mut output = path.to_string();
    for (name, value) in env::vars() {
        output = replace_env_token(&output, &name, &value);
    }
    output
}

fn replace_env_token(input: &str, name: &str, value: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    let token = format!("%{}%", name);

    while let Some(index) = rest.to_ascii_lowercase().find(&token.to_ascii_lowercase()) {
        output.push_str(&rest[..index]);
        output.push_str(value);
        rest = &rest[index + token.len()..];
    }
    output.push_str(rest);
    output
}

fn find_on_path(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?;
    env::var_os("PATH")?
        .to_string_lossy()
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(PathBuf::from)
        .map(|folder| folder.join(file_name))
        .find(|candidate| candidate.exists())
}

fn has_path_separator(path: &Path) -> bool {
    path.components().count() > 1
}

fn is_shell_item(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    ext == "lnk" || ext == "url"
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn file_stem(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn unique_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        if value.trim().is_empty() || unique.iter().any(|item| item == &value) {
            continue;
        }
        unique.push(value);
    }
    unique
}

fn dedupe_candidates(candidates: Vec<LaunchTarget>) -> Vec<LaunchTarget> {
    let mut unique = Vec::new();
    for candidate in candidates {
        let key = candidate.path.to_string_lossy().to_ascii_lowercase();
        if unique.iter().any(|item: &LaunchTarget| {
            item.path.to_string_lossy().to_ascii_lowercase() == key
                && item.arguments == candidate.arguments
                && item.use_shell == candidate.use_shell
        }) {
            continue;
        }
        unique.push(candidate);
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{LayoutRect, WindowStatePreference};

    fn app(display_name: &str, process_name: &str) -> AppConfig {
        AppConfig {
            id: "app".into(),
            display_name: display_name.into(),
            executable_path: None,
            arguments: vec![],
            working_directory: None,
            process_name: Some(process_name.into()),
            title_rule: None,
            class_name: None,
            target_monitor_id: None,
            layout: LayoutRect::default(),
            window_state: WindowStatePreference::Normal,
            launch_delay_seconds: 0,
            detection_timeout_seconds: 1,
            retry_interval_ms: 100,
            launch_if_missing: true,
            move_if_running: true,
            force_resize: true,
            apply_to_all_matching_windows: false,
            restore_if_minimized: true,
            pull_hidden_windows: true,
            wake_running_process: true,
            allow_empty_title: false,
            notes: None,
        }
    }

    #[test]
    fn app_path_names_adds_exe_extension() {
        assert_eq!(
            app_path_names("GitHubDesktop"),
            vec!["GitHubDesktop", "GitHubDesktop.exe"]
        );
        assert_eq!(
            app_path_names("GitHubDesktop.exe"),
            vec!["GitHubDesktop.exe"]
        );
    }

    #[test]
    fn known_candidates_include_github_desktop_squirrel_folder() {
        let candidates = known_install_candidates(
            &app("GitHub Desktop", "GitHubDesktop.exe"),
            "GitHubDesktop.exe",
        );
        assert!(candidates.iter().any(|candidate| {
            candidate
                .path
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("githubdesktop")
        }));
    }

    #[test]
    fn start_menu_match_accepts_display_name_or_process_stem() {
        assert!(installed_name_matches(
            "GitHub Desktop",
            &app("GitHub", "GitHubDesktop.exe"),
            "GitHubDesktop.exe"
        ));
        assert!(!installed_name_matches(
            "GitHub CLI",
            &app("GitHub", "GitHubDesktop.exe"),
            "GitHubDesktop.exe"
        ));
    }

    #[test]
    fn github_aliases_use_desktop_install_candidates() {
        let candidates = known_install_candidates(&app("GitHub", "GitHub.exe"), "GitHub.exe");
        assert!(candidates.iter().any(|candidate| {
            candidate
                .path
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("githubdesktop")
        }));
    }
}
