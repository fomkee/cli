#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(super) use linux::entry;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(super) use macos::entry;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use windows::entry;

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(super) use unsupported::entry;
