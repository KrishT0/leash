//! The administrator token check.
//!
//! Stopping most services needs an elevated process. Leash runs elevated from
//! the scheduled task, but it also runs from a normal shell during
//! development, so the app has to know which one it is.

use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::Error;

#[derive(Debug, thiserror::Error)]
pub enum ElevationError {
    #[error("cannot read the process token: {source}")]
    Windows {
        #[source]
        source: Error,
    },
}

/// True when this process runs with the administrator token.
///
/// The caller decides what to do. Without elevation the stop actions must be
/// turned off, while scanning and the config still work.
pub fn is_elevated() -> Result<bool, ElevationError> {
    let mut token = HANDLE(std::ptr::null_mut());
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut returned = 0u32;

    // SAFETY: GetCurrentProcess returns a pseudo handle that must not be
    // closed, and the out parameter points at a live local.
    let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if let Err(source) = opened {
        return Err(ElevationError::Windows { source });
    }

    // SAFETY: the token handle is open, and the buffer is exactly the size of
    // TOKEN_ELEVATION, which is what TokenElevation writes.
    let answer = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some(std::ptr::from_mut(&mut elevation).cast::<c_void>()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    };

    // SAFETY: the handle came from OpenProcessToken and is closed here, once,
    // on both the success and the failure path.
    unsafe {
        let _ = CloseHandle(token);
    }

    answer.map_err(|source| ElevationError::Windows { source })?;

    Ok(elevation.TokenIsElevated != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The tests below read the real token of this process.
    //     cargo test -p leash-engine -- --ignored

    #[test]
    #[ignore = "reads the real token. Run it from a normal, not elevated, shell"]
    fn should_report_a_normal_shell_as_not_elevated() {
        assert!(!is_elevated().expect("elevation"));
    }
}
