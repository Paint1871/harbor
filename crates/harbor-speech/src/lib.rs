use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DictationState {
    Idle,
    Listening,
    Transcribing,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictationEvent {
    pub state: DictationState,
    pub copy: &'static str,
}

pub fn begin() -> DictationEvent {
    DictationEvent {
        state: DictationState::Listening,
        copy: "Listening · release fn to send",
    }
}

pub fn engine_available() -> bool {
    false
}

pub fn start() -> DictationEvent {
    if engine_available() {
        begin()
    } else {
        silence_timeout()
    }
}

pub fn end() -> DictationEvent {
    DictationEvent {
        state: DictationState::Idle,
        copy: "",
    }
}

pub fn model_missing() -> DictationEvent {
    DictationEvent {
        state: DictationState::Error,
        copy: "Whisper model is not bundled in this build",
    }
}

pub fn silence_timeout() -> DictationEvent {
    DictationEvent {
        state: DictationState::Error,
        copy: "The microphone isn't delivering audio",
    }
}

pub fn ignore_phantom_tap(duration_ms: u64) -> bool {
    duration_ms < 150
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationDevice {
    pub id: String,
    pub label: String,
    pub selected: bool,
}

/// Local capture devices. Empty on unsupported hosts; never requires network.
pub fn list_devices() -> Vec<DictationDevice> {
    platform_devices()
}

#[cfg(target_os = "macos")]
fn platform_devices() -> Vec<DictationDevice> {
    macos::input_devices()
}

#[cfg(windows)]
fn platform_devices() -> Vec<DictationDevice> {
    windows::input_devices()
}

#[cfg(not(any(target_os = "macos", windows)))]
fn platform_devices() -> Vec<DictationDevice> {
    linux::input_devices()
}

fn mark_selected(
    mut devices: Vec<DictationDevice>,
    selected_id: Option<&str>,
) -> Vec<DictationDevice> {
    if devices.is_empty() {
        return devices;
    }
    let chosen = selected_id
        .and_then(|id| devices.iter().position(|device| device.id == id))
        .or_else(|| {
            devices.iter().position(|device| {
                let label = device.label.to_ascii_lowercase();
                !label.contains("loopback") && !label.contains("continuity")
            })
        })
        .unwrap_or(0);
    if let Some(device) = devices.get_mut(chosen) {
        device.selected = true;
    }
    devices
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{DictationDevice, mark_selected};
    use std::ffi::c_void;
    use std::mem;
    use std::ptr;

    type AudioObjectId = u32;
    type OsStatus = i32;

    #[repr(C)]
    struct AudioObjectPropertyAddress {
        selector: u32,
        scope: u32,
        element: u32,
    }

    const SYSTEM_OBJECT: AudioObjectId = 1;
    const ELEMENT_MAIN: u32 = 0;
    const UTF8: u32 = 0x0800_0100;

    fn fourcc(code: &[u8; 4]) -> u32 {
        u32::from_be_bytes(*code)
    }

    #[link(name = "CoreAudio", kind = "framework")]
    unsafe extern "C" {
        fn AudioObjectGetPropertyDataSize(
            id: AudioObjectId,
            address: *const AudioObjectPropertyAddress,
            qualifier_size: u32,
            qualifier: *const c_void,
            out_size: *mut u32,
        ) -> OsStatus;
        fn AudioObjectGetPropertyData(
            id: AudioObjectId,
            address: *const AudioObjectPropertyAddress,
            qualifier_size: u32,
            qualifier: *const c_void,
            io_size: *mut u32,
            out_data: *mut c_void,
        ) -> OsStatus;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *const c_void);
        fn CFStringGetCString(
            the_string: *const c_void,
            buffer: *mut u8,
            buffer_size: isize,
            encoding: u32,
        ) -> u8;
    }

    fn property(selector: u32, scope: u32) -> AudioObjectPropertyAddress {
        AudioObjectPropertyAddress {
            selector,
            scope,
            element: ELEMENT_MAIN,
        }
    }

    fn get_u32(id: AudioObjectId, address: &AudioObjectPropertyAddress) -> Option<u32> {
        let mut size = mem::size_of::<u32>() as u32;
        let mut value = 0u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                id,
                address,
                0,
                ptr::null(),
                &mut size,
                &mut value as *mut u32 as *mut c_void,
            )
        };
        (status == 0).then_some(value)
    }

    fn has_input_streams(id: AudioObjectId) -> bool {
        let address = property(fourcc(b"stm#"), fourcc(b"inpt"));
        let mut size = 0u32;
        let status =
            unsafe { AudioObjectGetPropertyDataSize(id, &address, 0, ptr::null(), &mut size) };
        status == 0 && size > 0
    }

    fn device_name(id: AudioObjectId) -> Option<String> {
        let address = property(fourcc(b"lnam"), fourcc(b"glob"));
        let mut size = mem::size_of::<*const c_void>() as u32;
        let mut cf: *const c_void = ptr::null();
        let status = unsafe {
            AudioObjectGetPropertyData(
                id,
                &address,
                0,
                ptr::null(),
                &mut size,
                &mut cf as *mut *const c_void as *mut c_void,
            )
        };
        if status != 0 || cf.is_null() {
            return None;
        }
        let mut buffer = [0u8; 256];
        let ok =
            unsafe { CFStringGetCString(cf, buffer.as_mut_ptr(), buffer.len() as isize, UTF8) };
        unsafe { CFRelease(cf) };
        if ok == 0 {
            return None;
        }
        let end = buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(buffer.len());
        String::from_utf8(buffer[..end].to_vec()).ok()
    }

    pub fn input_devices() -> Vec<DictationDevice> {
        let address = property(fourcc(b"dev#"), fourcc(b"glob"));
        let mut size = 0u32;
        let status = unsafe {
            AudioObjectGetPropertyDataSize(SYSTEM_OBJECT, &address, 0, ptr::null(), &mut size)
        };
        if status != 0 || size == 0 {
            return Vec::new();
        }
        let count = size as usize / mem::size_of::<AudioObjectId>();
        let mut ids = vec![0u32; count];
        let mut io_size = size;
        let status = unsafe {
            AudioObjectGetPropertyData(
                SYSTEM_OBJECT,
                &address,
                0,
                ptr::null(),
                &mut io_size,
                ids.as_mut_ptr() as *mut c_void,
            )
        };
        if status != 0 {
            return Vec::new();
        }
        let default_id = get_u32(SYSTEM_OBJECT, &property(fourcc(b"dIn "), fourcc(b"glob")))
            .map(|id| id.to_string());
        let mut devices = Vec::new();
        for id in ids {
            if !has_input_streams(id) {
                continue;
            }
            let label = device_name(id).unwrap_or_else(|| format!("Input {id}"));
            devices.push(DictationDevice {
                id: id.to_string(),
                label,
                selected: false,
            });
        }
        mark_selected(devices, default_id.as_deref())
    }
}

#[cfg(windows)]
mod windows {
    use super::{DictationDevice, mark_selected};

    #[repr(C)]
    struct WaveInCapsW {
        w_mid: u16,
        w_pid: u16,
        v_driver_version: u32,
        sz_pname: [u16; 32],
        dw_formats: u32,
        w_channels: u16,
        w_reserved1: u16,
    }

    #[link(name = "winmm")]
    unsafe extern "system" {
        fn waveInGetNumDevs() -> u32;
        fn waveInGetDevCapsW(device_id: usize, caps: *mut WaveInCapsW, size: u32) -> u32;
    }

    pub fn input_devices() -> Vec<DictationDevice> {
        let count = unsafe { waveInGetNumDevs() };
        let mut devices = Vec::new();
        for index in 0..count {
            let mut caps = WaveInCapsW {
                w_mid: 0,
                w_pid: 0,
                v_driver_version: 0,
                sz_pname: [0; 32],
                dw_formats: 0,
                w_channels: 0,
                w_reserved1: 0,
            };
            let status = unsafe {
                waveInGetDevCapsW(
                    index as usize,
                    &mut caps,
                    std::mem::size_of::<WaveInCapsW>() as u32,
                )
            };
            if status != 0 {
                continue;
            }
            let end = caps
                .sz_pname
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(caps.sz_pname.len());
            let label = String::from_utf16_lossy(&caps.sz_pname[..end]);
            if label.is_empty() {
                continue;
            }
            devices.push(DictationDevice {
                id: index.to_string(),
                label,
                selected: false,
            });
        }
        mark_selected(devices, Some("0"))
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod linux {
    use super::{DictationDevice, mark_selected};
    use std::fs;

    pub fn input_devices() -> Vec<DictationDevice> {
        let Ok(text) = fs::read_to_string("/proc/asound/cards") else {
            return Vec::new();
        };
        let mut devices = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty()
                || !line
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| byte.is_ascii_digit())
            {
                continue;
            }
            let mut parts = line.splitn(2, '[');
            let id = parts.next().unwrap_or("").trim().to_string();
            let rest = parts.next().unwrap_or("");
            let label = rest.split(']').next().unwrap_or(rest).trim().to_string();
            if id.is_empty() || label.is_empty() {
                continue;
            }
            devices.push(DictationDevice {
                id,
                label,
                selected: false,
            });
        }
        mark_selected(devices, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fail_closed_copy_and_anti_accidental() {
        assert_eq!(begin().copy, "Listening · release fn to send");
        assert_eq!(
            silence_timeout().copy,
            "The microphone isn't delivering audio"
        );
        assert!(ignore_phantom_tap(120));
        assert!(!ignore_phantom_tap(200));
        assert_eq!(start().state, DictationState::Error);
        assert_eq!(end().state, DictationState::Idle);
    }

    #[test]
    fn devices_list_is_local_and_well_formed() {
        let devices = list_devices();
        let selected = devices.iter().filter(|device| device.selected).count();
        assert!(selected <= 1);
        for device in &devices {
            assert!(!device.id.is_empty());
            assert!(!device.label.is_empty());
        }
        let marked = mark_selected(
            vec![
                DictationDevice {
                    id: "1".into(),
                    label: "Loopback".into(),
                    selected: false,
                },
                DictationDevice {
                    id: "2".into(),
                    label: "Desk mic".into(),
                    selected: false,
                },
            ],
            None,
        );
        assert!(marked[1].selected);
        assert!(!marked[0].selected);
    }
}
