use std::{ffi::c_void, thread};

use tauri::AppHandle;
use windows::{
    core::w,
    Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0},
        System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject},
    },
};

use crate::performance;

pub struct SingleInstanceGuard {
    mutex: HANDLE,
    activation_event: Option<HANDLE>,
}

pub fn acquire() -> Option<SingleInstanceGuard> {
    let handle = unsafe { CreateMutexW(None, true, w!("Local\\WindowAutoLayout")) }.ok()?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        signal_existing_instance();
        unsafe {
            let _ = CloseHandle(handle);
        }
        return None;
    }

    let activation_event = unsafe {
        CreateEventW(
            None,
            false,
            false,
            w!("Local\\WindowAutoLayout.ShowMainWindow"),
        )
        .ok()
    };
    Some(SingleInstanceGuard {
        mutex: handle,
        activation_event,
    })
}

impl SingleInstanceGuard {
    pub fn activation_event_address(&self) -> Option<usize> {
        self.activation_event.map(|event| event.0 as usize)
    }
}

pub fn start_activation_listener(event_address: usize, app: AppHandle) -> std::io::Result<()> {
    thread::Builder::new()
        .name("windowautolayout-activation".into())
        .spawn(move || {
            performance::lower_current_thread_priority();
            let event = HANDLE(event_address as *mut c_void);
            loop {
                if unsafe { WaitForSingleObject(event, u32::MAX) } != WAIT_OBJECT_0 {
                    break;
                }
                crate::show_main_window(&app);
            }
        })
        .map(|_| ())
}

fn signal_existing_instance() {
    let Ok(event) = (unsafe {
        CreateEventW(
            None,
            false,
            false,
            w!("Local\\WindowAutoLayout.ShowMainWindow"),
        )
    }) else {
        return;
    };
    unsafe {
        let _ = SetEvent(event);
        let _ = CloseHandle(event);
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(event) = self.activation_event {
                let _ = CloseHandle(event);
            }
            let _ = CloseHandle(self.mutex);
        }
    }
}
