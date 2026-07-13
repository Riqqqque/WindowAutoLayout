use std::path::Path;

use windows::{
    core::PWSTR,
    Win32::{
        Foundation::CloseHandle,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    },
};

use crate::errors::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

pub fn list_processes() -> AppResult<Vec<ProcessInfo>> {
    let snapshot = unsafe {
        CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|err| AppError::Windows(err.message().to_string()))?
    };

    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    let mut processes = Vec::new();
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry).is_ok() };

    while has_entry {
        let name = wide_to_string(&entry.szExeFile);
        let pid = entry.th32ProcessID;
        processes.push(ProcessInfo { pid, name });
        has_entry = unsafe { Process32NextW(snapshot, &mut entry).is_ok() };
    }

    unsafe {
        let _ = CloseHandle(snapshot);
    }

    Ok(processes)
}

pub fn query_process_name(pid: u32) -> String {
    query_process_path(pid)
        .as_deref()
        .and_then(file_name_from_path)
        .or_else(|| {
            list_processes()
                .ok()
                .and_then(|processes| processes.into_iter().find(|process| process.pid == pid))
                .map(|process| process.name)
        })
        .unwrap_or_default()
}

pub fn query_process_path(pid: u32) -> Option<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    unsafe {
        let _ = CloseHandle(handle);
    }

    if result.is_ok() && length > 0 {
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    } else {
        None
    }
}

pub fn file_name_from_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
}

pub fn windows_app_package_family(path: &str) -> Option<String> {
    let normalized = path.replace('/', "\\");
    let lower = normalized.to_ascii_lowercase();
    let marker = "\\windowsapps\\";
    let start = lower.find(marker)? + marker.len();
    let package_full_name = normalized[start..].split('\\').next()?;
    let (name_and_version, publisher_id) = package_full_name.rsplit_once("__")?;
    let package_name = name_and_version.split('_').next()?;
    if package_name.is_empty() || publisher_id.is_empty() {
        return None;
    }

    Some(format!("{package_name}_{publisher_id}"))
}

pub fn same_windows_app_package(left: &str, right: &str) -> bool {
    windows_app_package_family(left)
        .zip(windows_app_package_family(right))
        .map(|(left, right)| left.eq_ignore_ascii_case(&right))
        .unwrap_or(false)
}

fn wide_to_string(buffer: &[u16]) -> String {
    let len = buffer
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_package_identity_survives_versioned_folder_changes() {
        let old = r"C:\Program Files\WindowsApps\OpenAI.Codex_26.623.5546.0_x64__2p2nqsd0c76g0\app\Codex.exe";
        let new = r"C:\Program Files\WindowsApps\OpenAI.Codex_26.707.3563.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe";

        assert_eq!(
            windows_app_package_family(old).as_deref(),
            Some("OpenAI.Codex_2p2nqsd0c76g0")
        );
        assert!(same_windows_app_package(old, new));
        assert!(!same_windows_app_package(old, r"C:\Apps\Discord.exe"));
    }
}
