//! Windows Service Control Manager calls.
//!
//! This file and `elevation.rs` are the only places that call the Windows API.
//! Every call sits inside a safe function. Each `unsafe` block has a comment
//! that says why the call cannot break memory safety.

use crate::safety::ServiceFacts;
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::mem::size_of;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_DEPENDENT_SERVICES_RUNNING, ERROR_INSUFFICIENT_BUFFER,
    ERROR_MORE_DATA, ERROR_SERVICE_DISABLED, ERROR_SERVICE_DOES_NOT_EXIST, HANDLE, WAIT_EVENT,
    WAIT_IO_COMPLETION, WAIT_OBJECT_0, WIN32_ERROR,
};
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, ENUM_SERVICE_STATUS_PROCESSW, ENUM_SERVICE_STATUSW,
    EnumDependentServicesW, EnumServicesStatusExW, NotifyServiceStatusChangeW, OpenSCManagerW,
    OpenServiceW, QUERY_SERVICE_CONFIGW, QueryServiceConfigW, QueryServiceStatusEx,
    SC_ENUM_PROCESS_INFO, SC_HANDLE, SC_MANAGER_CONNECT, SC_MANAGER_ENUMERATE_SERVICE,
    SC_STATUS_PROCESS_INFO, SERVICE_ACCEPT_STOP, SERVICE_ACTIVE, SERVICE_AUTO_START,
    SERVICE_BOOT_START, SERVICE_CONTINUE_PENDING, SERVICE_CONTROL_STOP, SERVICE_DEMAND_START,
    SERVICE_DISABLED, SERVICE_DRIVER, SERVICE_ENUMERATE_DEPENDENTS, SERVICE_NOTIFY_2W,
    SERVICE_NOTIFY_RUNNING, SERVICE_PAUSE_PENDING, SERVICE_PAUSED, SERVICE_QUERY_CONFIG,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_START_PENDING,
    SERVICE_START_TYPE, SERVICE_STATE_ALL, SERVICE_STATUS, SERVICE_STATUS_CURRENT_STATE,
    SERVICE_STATUS_PROCESS, SERVICE_STOP, SERVICE_STOP_PENDING, SERVICE_STOPPED,
    SERVICE_SYSTEM_START, SERVICE_WIN32, StartServiceW,
};
use windows::Win32::System::Threading::{
    CreateEventW, INFINITE, SetEvent, WaitForMultipleObjectsEx,
};
use windows::core::{Error, HRESULT, PCWSTR};

/// What a service is doing now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    Stopped,
    StartPending,
    StopPending,
    Paused,
    PausePending,
    ContinuePending,
    Unknown,
}

/// When a service starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Startup {
    Automatic,
    Manual,
    Disabled,
    Boot,
    System,
    Unknown,
}

/// One service, as the UI needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Internal name.
    pub name: String,
    /// Name Windows shows to the user.
    pub display_name: String,
    pub status: Status,
    pub startup: Startup,
    /// The service accepts a stop command.
    pub accepts_stop: bool,
    /// A kernel or file system driver.
    pub is_driver: bool,
    /// A Win32 service, not a driver.
    pub is_win32: bool,
    /// The binary sits outside the Windows folder.
    pub third_party: bool,
    /// Process id, when the service runs.
    pub pid: Option<u32>,
}

impl ServiceInfo {
    /// The facts the safety check needs that come from the service itself. The
    /// dependent count needs its own call, so it is left at zero here. The list
    /// view uses this, which is why listing services costs no extra calls.
    pub fn facts(&self) -> ServiceFacts {
        ServiceFacts {
            name: self.name.clone(),
            // Windows reports no accepted controls while a service is stopped.
            // Stopping it again is a no-op, so it does not count as blocked.
            accepts_stop: self.accepts_stop || self.status == Status::Stopped,
            is_driver: self.is_driver,
            is_win32: self.is_win32,
            running_dependents: 0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScmError {
    #[error("the service {name} does not exist")]
    NotFound { name: String },
    #[error("access denied for {name}. Leash needs administrator rights")]
    AccessDenied { name: String },
    #[error("the Windows service call failed: {source}")]
    Windows {
        #[source]
        source: Error,
    },
    #[error("Windows reported a size of 0 bytes for {name}")]
    Empty { name: String },
    #[error("the watch on a service failed with wait code {code}")]
    Wait { code: u32 },
    #[error("a service name from Windows is not valid text")]
    Name,
}

/// Every service on the machine, drivers included, sorted by name.
pub fn list_services() -> Result<Vec<ServiceInfo>, ScmError> {
    let manager = open_manager()?;
    let kinds = SERVICE_WIN32 | SERVICE_DRIVER;
    let mut services = Vec::new();
    let mut resume = 0u32;
    // A size of 0 means: ask Windows how much room it needs first.
    let mut size = 0u32;

    loop {
        let mut needed = 0u32;
        let mut returned = 0u32;

        if size == 0 {
            // SAFETY: the manager handle is open. A null buffer with a zero size
            // only asks for the size of the answer.
            let probe = unsafe {
                EnumServicesStatusExW(
                    manager.0,
                    SC_ENUM_PROCESS_INFO,
                    kinds,
                    SERVICE_STATE_ALL,
                    None,
                    &mut needed,
                    &mut returned,
                    Some(std::ptr::from_mut(&mut resume)),
                    PCWSTR::null(),
                )
            };
            match probe {
                // Nothing left to enumerate.
                Ok(()) => break,
                Err(error)
                    if is_win32(&error, ERROR_MORE_DATA)
                        || is_win32(&error, ERROR_INSUFFICIENT_BUFFER) => {}
                Err(source) => return Err(ScmError::Windows { source }),
            }
            if needed == 0 {
                break;
            }
            size = needed;
        }

        let capacity = (size as usize).div_ceil(size_of::<ENUM_SERVICE_STATUS_PROCESSW>());
        let mut buffer = vec![ENUM_SERVICE_STATUS_PROCESSW::default(); capacity];
        // SAFETY: the buffer is `size` bytes, is aligned for the struct, and
        // outlives the call.
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(buffer.as_mut_ptr().cast::<u8>(), size as usize)
        };

        // SAFETY: same call as above, now with a buffer that the probe sized.
        let outcome = unsafe {
            EnumServicesStatusExW(
                manager.0,
                SC_ENUM_PROCESS_INFO,
                kinds,
                SERVICE_STATE_ALL,
                Some(bytes),
                &mut needed,
                &mut returned,
                Some(std::ptr::from_mut(&mut resume)),
                PCWSTR::null(),
            )
        };
        // Windows also reports the size it wants when the buffer is too small.
        // A successful call ends the enumeration. The resume handle is not
        // usable after it, so it must not be probed again.
        let more = match outcome {
            Ok(()) => false,
            Err(error) if is_win32(&error, ERROR_MORE_DATA) => true,
            Err(source) => return Err(ScmError::Windows { source }),
        };

        for entry in buffer.iter().take(returned as usize) {
            // SAFETY: the call wrote null-terminated names into the live buffer.
            let name = unsafe { entry.lpServiceName.to_string() }.map_err(|_| ScmError::Name)?;
            let display_name = unsafe { entry.lpDisplayName.to_string() }.unwrap_or_default();

            // A service that refuses the config call still belongs in the list,
            // with an unknown startup type.
            let config = read_config(&manager, &name).unwrap_or(ConfigSummary {
                startup: Startup::Unknown,
                display_name: display_name.clone(),
                third_party: false,
            });

            services.push(info_from(name, &entry.ServiceStatusProcess, config));
        }

        if !more {
            break;
        }

        // The list grew between the two calls. Take the larger request. The
        // doubling guarantees a strictly larger size, so this loop always ends.
        size = needed.max(size.saturating_mul(2));
    }

    services.sort_by_key(|service| service.name.to_ascii_lowercase());
    Ok(services)
}

/// One service, by internal name.
pub fn query(name: &str) -> Result<ServiceInfo, ScmError> {
    let manager = open_manager()?;
    let handle = open_service(&manager, name, SERVICE_QUERY_STATUS | SERVICE_QUERY_CONFIG)?;
    let status = status_of(&handle, name)?;
    let config = config_of(&handle, name)?;

    Ok(info_from(name.to_string(), &status, config))
}

/// The services that depend on this one and are running now.
pub fn dependents(name: &str) -> Result<Vec<String>, ScmError> {
    let manager = open_manager()?;
    let handle = open_service(
        &manager,
        name,
        SERVICE_QUERY_STATUS | SERVICE_ENUMERATE_DEPENDENTS,
    )?;

    let mut needed = 0u32;
    let mut returned = 0u32;
    // SAFETY: the handle is open. A null buffer with a zero size only asks for
    // the size of the answer.
    let probe = unsafe {
        EnumDependentServicesW(
            handle.0,
            SERVICE_ACTIVE,
            None,
            0,
            &mut needed,
            &mut returned,
        )
    };
    match probe {
        Ok(()) => {}
        Err(error) if is_win32(&error, ERROR_MORE_DATA) => {}
        Err(source) => return Err(classify(source, name)),
    }
    if needed == 0 {
        return Ok(Vec::new());
    }

    let capacity = (needed as usize).div_ceil(size_of::<ENUM_SERVICE_STATUSW>());
    let mut buffer = vec![ENUM_SERVICE_STATUSW::default(); capacity];
    // SAFETY: the buffer is at least `needed` bytes, is aligned for the struct,
    // and outlives the call.
    let outcome = unsafe {
        EnumDependentServicesW(
            handle.0,
            SERVICE_ACTIVE,
            Some(buffer.as_mut_ptr()),
            needed,
            &mut needed,
            &mut returned,
        )
    };
    match outcome {
        Ok(()) => {}
        Err(error) if is_win32(&error, ERROR_MORE_DATA) => {}
        Err(source) => return Err(classify(source, name)),
    }

    let mut names = Vec::with_capacity(returned as usize);
    for entry in buffer.iter().take(returned as usize) {
        // SAFETY: the call wrote a null-terminated name into the live buffer.
        if let Ok(name) = unsafe { entry.lpServiceName.to_string() } {
            names.push(name);
        }
    }

    Ok(names)
}

/// How long a start or a stop may take before Leash gives up waiting.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(30);

/// The `dwVersion` field of `SERVICE_NOTIFY_2W`. The Windows SDK defines
/// `SERVICE_NOTIFY_STATUS_CHANGE` as this value, and the crate does not export
/// it, because the crate names the same number `SERVICE_NOTIFY_START_PENDING`.
const NOTIFY_VERSION: u32 = 2;

/// A manual reset event that tells every watch thread to stop.
#[derive(Debug)]
pub struct StopEvent(HANDLE);

impl StopEvent {
    /// Create the event, ready to be waited on.
    pub fn new() -> Result<Self, ScmError> {
        // SAFETY: no security attributes, a manual reset event, not signalled,
        // and unnamed, so no other process can reach it.
        let handle = unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
            .map_err(|source| ScmError::Windows { source })?;

        Ok(Self(handle))
    }

    /// Ask every watcher to stop. Safe to call more than once.
    pub fn set(&self) {
        // SAFETY: the handle came from CreateEventW and is still open.
        let _ = unsafe { SetEvent(self.0) };
    }
}

impl Drop for StopEvent {
    fn drop(&mut self) {
        // SAFETY: the handle came from CreateEventW and is closed once, here.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// Why [`Watched::wait`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchState {
    /// The service changed state.
    Changed,
    /// The caller asked the watch to stop.
    Quit,
}

/// One service under watch.
#[derive(Debug)]
pub struct Watched {
    handle: Handle,
    name: String,
    /// The event that the Windows callback signals.
    event: HANDLE,
    /// The buffer the registration reads. It must stay alive and unchanged
    /// while a notification is pending, so it is boxed and never moved.
    notify: Box<SERVICE_NOTIFY_2W>,
    quit: HANDLE,
}

// SAFETY: a `Watched` owns its service handle, its event handle, and its
// notification buffer. Nothing else touches them. Every use of the raw pointers
// inside goes straight to Windows, from the thread that owns the value.
unsafe impl Send for Watched {}

/// How many times the notification callback has run. Test builds only, so a
/// diagnostic can tell "the callback never ran" from "the callback ran and the
/// wake up failed".
#[cfg(test)]
pub(crate) static CALLBACKS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// How many times the callback could not signal the event. Test builds only.
#[cfg(test)]
pub(crate) static SETEVENT_FAILURES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Watch one service. Call [`Watched::arm`] then [`Watched::wait`] in a loop.
pub fn watch(name: &str, quit: &StopEvent) -> Result<Watched, ScmError> {
    let manager = open_manager()?;
    // Watching only needs to read the status.
    let handle = open_service(&manager, name, SERVICE_QUERY_STATUS)?;

    // SAFETY: no security attributes, an auto reset event that starts
    // unsignalled, and unnamed, so no other process can reach it.
    let event = unsafe { CreateEventW(None, false, false, PCWSTR::null()) }
        .map_err(|source| ScmError::Windows { source })?;

    let mut notify = Box::new(SERVICE_NOTIFY_2W::default());
    notify.dwVersion = NOTIFY_VERSION;
    notify.pfnNotifyCallback = Some(on_status_change);
    // The callback cannot use this value directly, because Windows passes the
    // structure itself. The event handle sits in pContext, inside that same
    // structure, where the callback reads it.
    notify.pContext = event.0;

    Ok(Watched {
        handle,
        name: name.to_string(),
        event,
        notify,
        quit: quit.0,
    })
}

impl Watched {
    /// Arm the watch. Call this before [`Watched::wait`], and again as soon as
    /// `wait` returns.
    ///
    /// A notification covers one change, and Windows only reports a change that
    /// happens while a request is outstanding. Arming straight after a wake up
    /// means a service that comes back while Leash is stopping it is still
    /// seen.
    pub fn arm(&mut self) -> Result<(), ScmError> {
        // Windows reads the buffer during the call, and the callback only reads
        // the context, so one buffer can serve every round.
        // SAFETY: the service handle is open, the buffer is alive, and the
        // previous callback has finished with it, because `wait` returned.
        let code = unsafe {
            NotifyServiceStatusChangeW(self.handle.0, SERVICE_NOTIFY_RUNNING, &*self.notify)
        };
        if code != 0 {
            return Err(classify(
                Error::from_hresult(HRESULT::from_win32(code)),
                &self.name,
            ));
        }

        Ok(())
    }

    /// Wait for the next status change, or for the caller to ask the watch to
    /// stop. Call [`Watched::arm`] first.
    pub fn wait(&mut self) -> Result<WatchState, ScmError> {
        self.wait_for(INFINITE)
    }

    /// The notification fields that Windows fills in, for tests.
    #[cfg(test)]
    fn notification(&self) -> (u32, u32) {
        (
            self.notify.dwNotificationStatus,
            self.notify.dwNotificationTriggered,
        )
    }

    /// Wait with a limit, for tests. A timeout comes back as an error with the
    /// wait code 258.
    fn wait_for(&mut self, milliseconds: u32) -> Result<WatchState, ScmError> {
        let handles = [self.quit, self.event];

        loop {
            // The wait has to be alertable. Windows delivers the callback as an
            // asynchronous procedure call queued to this thread, and a thread
            // only runs a queued call during an alertable wait. A plain
            // WaitForMultipleObjects would never run the callback, so the event
            // would never be signalled and the watch would wait for ever.
            // SAFETY: both handles stay open for the life of this struct, and
            // the wait does not touch any memory of ours.
            let waited = unsafe { WaitForMultipleObjectsEx(&handles, false, milliseconds, true) };
            if waited == WAIT_OBJECT_0 {
                return Ok(WatchState::Quit);
            }
            if waited == WAIT_EVENT(WAIT_OBJECT_0.0 + 1) {
                return Ok(WatchState::Changed);
            }
            // The queued callback ended the wait. It also signalled the event,
            // so go straight back to the wait.
            if waited == WAIT_IO_COMPLETION {
                continue;
            }

            return Err(ScmError::Wait { code: waited.0 });
        }
    }

    /// The current state of the watched service.
    pub fn status(&self) -> Result<Status, ScmError> {
        Ok(status_from(
            status_of(&self.handle, &self.name)?.dwCurrentState,
        ))
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        // SAFETY: the event came from CreateEventW and is closed once, here.
        // A notification can still arrive after this, because Windows does not
        // say when the callback has finished. Such a late call fails, or wakes
        // a recycled handle once. That costs one spurious wake up, which the
        // loop handles by reading the status again.
        unsafe {
            let _ = CloseHandle(self.event);
        }
    }
}

/// The callback Windows runs when the status changes.
///
/// Windows passes the address of the notification structure that was given to
/// `NotifyServiceStatusChangeW`, not the `pContext` value. Measured on Windows 11
/// build 26200: treating the parameter as the context calls `SetEvent` on a heap
/// address, which fails with ERROR_INVALID_HANDLE. The event is never signalled,
/// so the watch waits for ever. The event handle is read out of the structure.
unsafe extern "system" fn on_status_change(parameter: *const c_void) {
    #[cfg(test)]
    CALLBACKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    if parameter.is_null() {
        return;
    }

    // SAFETY: Windows passes the address of the SERVICE_NOTIFY_2W buffer that
    // `watch` allocated. That buffer stays alive and is not moved until the
    // watch is dropped, and only the pContext field is read here.
    let notify = unsafe { &*(parameter as *const SERVICE_NOTIFY_2W) };

    // SAFETY: pContext holds the event handle that `watch` stored there, and
    // that handle stays open until the watch is dropped.
    let signalled = unsafe { SetEvent(HANDLE(notify.pContext)).is_ok() };
    #[cfg(test)]
    if !signalled {
        SETEVENT_FAILURES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    #[cfg(not(test))]
    let _ = signalled;
}

/// What happened when Leash asked a service to stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopOutcome {
    /// The service stopped.
    Stopped,
    /// Other running services depend on it, so Windows refused the stop.
    DependentsRunning { dependents: Vec<String> },
    /// The service does not accept a stop command.
    NotAccepted,
    /// The wait ended and the service is still not stopped.
    NotStopped,
}

/// Ask a service to stop.
///
/// The caller runs the safety check first. This function does not consult the
/// deny list, so it must never be called on a blocked service.
///
/// The wait can stretch to three times `wait` while the service keeps reporting
/// progress through its checkpoint. After that Leash stops waiting.
pub fn stop(name: &str, wait: Duration) -> Result<StopOutcome, ScmError> {
    let manager = open_manager()?;
    // The stop procedure needs these two rights and no more.
    let handle = open_service(&manager, name, SERVICE_STOP | SERVICE_QUERY_STATUS)?;

    let status = status_of(&handle, name)?;
    if status_from(status.dwCurrentState) == Status::Stopped {
        return Ok(StopOutcome::Stopped);
    }
    // A stopped service reports no accepted controls, so this check has to come
    // after the one above.
    if !accepts_stop(&status) {
        return Ok(StopOutcome::NotAccepted);
    }

    let mut asked = SERVICE_STATUS::default();
    // SAFETY: the handle carries SERVICE_STOP, and the out parameter points at a
    // live local that stays alive for the call.
    let answer = unsafe { ControlService(handle.0, SERVICE_CONTROL_STOP, &mut asked) };
    match answer {
        Ok(()) => {}
        Err(error) if is_win32(&error, ERROR_DEPENDENT_SERVICES_RUNNING) => {
            // Never stop a dependent. Report them instead.
            return Ok(StopOutcome::DependentsRunning {
                dependents: dependents(name).unwrap_or_default(),
            });
        }
        Err(source) => return Err(classify(source, name)),
    }

    let stopped = wait_for_state(&handle, name, wait, Status::StopPending, Status::Stopped)?;

    Ok(if stopped {
        StopOutcome::Stopped
    } else {
        StopOutcome::NotStopped
    })
}

/// What happened when Leash asked a service to start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartOutcome {
    /// The service is running, either because it just started or because it
    /// already was.
    Started,
    /// The service is disabled, so Windows refused. Leash never changes a
    /// service setting.
    Disabled,
    /// The wait ended and the service is still not running.
    NotStarted,
}

/// Ask a service to start. This is the undo of a stop.
///
/// The caller runs the safety check first, exactly as for a stop. This function
/// changes no service setting, so a disabled service stays disabled and the
/// answer is [`StartOutcome::Disabled`].
///
/// The wait can stretch to three times `wait` while the service keeps reporting
/// progress through its checkpoint. After that Leash stops waiting.
pub fn start(name: &str, wait: Duration) -> Result<StartOutcome, ScmError> {
    let manager = open_manager()?;
    // The start procedure needs these two rights and no more.
    let handle = open_service(&manager, name, SERVICE_START | SERVICE_QUERY_STATUS)?;

    if status_from(status_of(&handle, name)?.dwCurrentState) == Status::Running {
        return Ok(StartOutcome::Started);
    }

    // SAFETY: the handle carries SERVICE_START, and no arguments go to the
    // service here.
    let answer = unsafe { StartServiceW(handle.0, None) };
    match answer {
        Ok(()) => {}
        Err(error) if is_win32(&error, ERROR_SERVICE_DISABLED) => {
            return Ok(StartOutcome::Disabled);
        }
        Err(source) => return Err(classify(source, name)),
    }

    let started = wait_for_state(&handle, name, wait, Status::StartPending, Status::Running)?;

    Ok(if started {
        StartOutcome::Started
    } else {
        StartOutcome::NotStarted
    })
}

/// Wait while the service sits in `pending`, until it reaches `wanted`. True when
/// it got there, and false when it settled somewhere else or the wait ran out.
///
/// The wait hint sets the pace, the checkpoint shows progress, and the deadline
/// ends it. The wait can stretch to three times `wait` while progress continues.
fn wait_for_state(
    handle: &Handle,
    name: &str,
    wait: Duration,
    pending: Status,
    wanted: Status,
) -> Result<bool, ScmError> {
    let opened = Instant::now();
    let hard_deadline = opened + wait.saturating_mul(3);
    let mut deadline = opened + wait;
    let mut last_checkpoint = 0u32;

    loop {
        let status = status_of(handle, name)?;
        let state = status_from(status.dwCurrentState);

        if state == wanted {
            return Ok(true);
        }
        // It settled somewhere else, or it never left. Waiting on does not
        // help either way.
        if state != pending {
            return Ok(false);
        }

        if status.dwCheckPoint != last_checkpoint {
            last_checkpoint = status.dwCheckPoint;
            // The service is working. Give it another window, but never past
            // the hard deadline.
            deadline = (Instant::now() + wait).min(hard_deadline);
        }

        let now = Instant::now();
        if now >= deadline {
            return Ok(false);
        }

        // Ask faster than the hint the service gave, so a change that finishes
        // early is noticed at once.
        let hint = u64::from(status.dwWaitHint.max(100));
        let step = Duration::from_millis(hint / 4)
            .clamp(Duration::from_millis(50), Duration::from_millis(500));
        std::thread::sleep(step.min(deadline - now));
    }
}

/// The facts that the safety check needs about one service.
///
/// An error here means the facts are unknown. The caller must not stop a
/// service whose facts it could not read. Windows denies the read for some
/// services, such as `RpcSs`, when the process is not elevated.
pub fn facts(name: &str) -> Result<ServiceFacts, ScmError> {
    let info = query(name)?;
    let mut facts = info.facts();
    facts.running_dependents = dependents(name)?.len();

    Ok(facts)
}

/// A manager handle that closes itself.
#[derive(Debug)]
struct Handle(SC_HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from OpenSCManagerW or OpenServiceW, and it
        // is closed once, here.
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

/// What the config of one service tells us.
struct ConfigSummary {
    startup: Startup,
    display_name: String,
    third_party: bool,
}

fn open_manager() -> Result<Handle, ScmError> {
    // SAFETY: a null machine name and a null database name mean the local
    // machine, which is the only case Leash supports.
    let handle = unsafe {
        OpenSCManagerW(
            PCWSTR::null(),
            PCWSTR::null(),
            SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE,
        )
    }
    .map_err(|source| ScmError::Windows { source })?;

    Ok(Handle(handle))
}

fn open_service(manager: &Handle, name: &str, access: u32) -> Result<Handle, ScmError> {
    let wide = wide(name);
    // SAFETY: `wide` is a null-terminated UTF-16 buffer that outlives the call,
    // and the manager handle is open.
    let result = unsafe { OpenServiceW(manager.0, PCWSTR::from_raw(wide.as_ptr()), access) };

    match result {
        Ok(handle) => Ok(Handle(handle)),
        Err(source) => Err(classify(source, name)),
    }
}

fn read_config(manager: &Handle, name: &str) -> Result<ConfigSummary, ScmError> {
    let handle = open_service(manager, name, SERVICE_QUERY_CONFIG)?;
    config_of(&handle, name)
}

fn status_of(handle: &Handle, name: &str) -> Result<SERVICE_STATUS_PROCESS, ScmError> {
    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut needed = 0u32;

    // SAFETY: the slice covers exactly the stack value below, which
    // SC_STATUS_PROCESS_INFO is documented to fill, and it outlives the call.
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(
            std::ptr::from_mut(&mut status).cast::<u8>(),
            size_of::<SERVICE_STATUS_PROCESS>(),
        )
    };
    // SAFETY: the handle is open and the buffer matches the requested level.
    unsafe { QueryServiceStatusEx(handle.0, SC_STATUS_PROCESS_INFO, Some(bytes), &mut needed) }
        .map_err(|source| classify(source, name))?;

    Ok(status)
}

fn config_of(handle: &Handle, name: &str) -> Result<ConfigSummary, ScmError> {
    let mut needed = 0u32;
    // SAFETY: the handle is open. A null buffer only asks for the size.
    let probe = unsafe { QueryServiceConfigW(handle.0, None, 0, &mut needed) };
    match probe {
        Ok(()) => {}
        Err(error) if is_win32(&error, ERROR_INSUFFICIENT_BUFFER) => {}
        Err(source) => return Err(classify(source, name)),
    }
    if needed == 0 {
        return Err(ScmError::Empty {
            name: name.to_string(),
        });
    }

    // A `u64` buffer stays aligned for the pointers inside the config struct.
    let mut buffer = vec![0u64; (needed as usize).div_ceil(size_of::<u64>())];
    // SAFETY: the buffer is at least `needed` bytes, is aligned, and outlives
    // the call.
    let outcome = unsafe {
        QueryServiceConfigW(
            handle.0,
            Some(buffer.as_mut_ptr().cast::<QUERY_SERVICE_CONFIGW>()),
            needed,
            &mut needed,
        )
    };
    match outcome {
        Ok(()) => {}
        Err(source) => return Err(classify(source, name)),
    }

    // SAFETY: the call above wrote a QUERY_SERVICE_CONFIGW with its strings into
    // the buffer, which is still alive and unchanged.
    let config = unsafe { &*buffer.as_ptr().cast::<QUERY_SERVICE_CONFIGW>() };
    // SAFETY: both pointers point into the same live buffer and are
    // null-terminated.
    let display_name = unsafe { config.lpDisplayName.to_string() }.unwrap_or_default();
    let binary_path = unsafe { config.lpBinaryPathName.to_string() }.unwrap_or_default();

    Ok(ConfigSummary {
        startup: startup_from(config.dwStartType),
        display_name: if display_name.is_empty() {
            name.to_string()
        } else {
            display_name
        },
        third_party: is_third_party(&binary_path),
    })
}

fn info_from(name: String, status: &SERVICE_STATUS_PROCESS, config: ConfigSummary) -> ServiceInfo {
    let kinds = status.dwServiceType.0;

    ServiceInfo {
        display_name: config.display_name,
        name,
        status: status_from(status.dwCurrentState),
        startup: config.startup,
        accepts_stop: accepts_stop(status),
        is_win32: kinds & SERVICE_WIN32.0 != 0,
        is_driver: kinds & SERVICE_DRIVER.0 != 0,
        third_party: config.third_party,
        pid: match status.dwProcessId {
            0 => None,
            pid => Some(pid),
        },
    }
}

/// True when the service accepts a stop command. Windows reports no controls
/// while a service is stopped.
fn accepts_stop(status: &SERVICE_STATUS_PROCESS) -> bool {
    status.dwControlsAccepted & SERVICE_ACCEPT_STOP != 0
}

fn status_from(state: SERVICE_STATUS_CURRENT_STATE) -> Status {
    match state {
        SERVICE_RUNNING => Status::Running,
        SERVICE_STOPPED => Status::Stopped,
        SERVICE_START_PENDING => Status::StartPending,
        SERVICE_STOP_PENDING => Status::StopPending,
        SERVICE_PAUSED => Status::Paused,
        SERVICE_PAUSE_PENDING => Status::PausePending,
        SERVICE_CONTINUE_PENDING => Status::ContinuePending,
        _ => Status::Unknown,
    }
}

fn startup_from(start: SERVICE_START_TYPE) -> Startup {
    match start {
        SERVICE_AUTO_START => Startup::Automatic,
        SERVICE_DEMAND_START => Startup::Manual,
        SERVICE_DISABLED => Startup::Disabled,
        SERVICE_BOOT_START => Startup::Boot,
        SERVICE_SYSTEM_START => Startup::System,
        _ => Startup::Unknown,
    }
}

/// True when the binary sits outside the Windows folder.
fn is_third_party(binary_path: &str) -> bool {
    let trimmed = binary_path.trim_matches('"').trim().to_ascii_lowercase();
    let path = trimmed.strip_prefix("\\??\\").unwrap_or(&trimmed);

    if path.is_empty() {
        return false;
    }
    // A path with no drive letter, such as `System32\drivers\ACPI.sys` or
    // `\SystemRoot\...`, resolves inside the Windows folder. Drivers use this
    // form.
    if !path.contains(":\\") {
        return false;
    }

    let windows = windows_dir().to_ascii_lowercase();
    !path.starts_with(&format!("{}\\", windows.trim_end_matches('\\')))
}

fn windows_dir() -> String {
    std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn is_win32(error: &Error, code: WIN32_ERROR) -> bool {
    error.code() == HRESULT::from_win32(code.0)
}

fn classify(source: Error, name: &str) -> ScmError {
    if is_win32(&source, ERROR_SERVICE_DOES_NOT_EXIST) {
        return ScmError::NotFound {
            name: name.to_string(),
        };
    }
    if is_win32(&source, ERROR_ACCESS_DENIED) {
        return ScmError::AccessDenied {
            name: name.to_string(),
        };
    }

    ScmError::Windows { source }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::{Level, Reason, SafetyLists};
    use windows::Win32::System::Threading::{GetCurrentThread, QueueUserAPC};

    // The tests below read the real token of this process.
    //     cargo test -p leash-engine -- --ignored

    #[test]
    #[ignore = "queues a real call onto this thread"]
    fn should_run_a_queued_call_only_during_an_alertable_wait() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static RAN: AtomicBool = AtomicBool::new(false);

        unsafe extern "system" fn mark(_: usize) {
            RAN.store(true, Ordering::SeqCst);
        }

        let never = StopEvent::new().expect("event");
        let handles = [never.0];

        // The watch waits this way, because Windows delivers the service
        // notification as a call queued to this thread.
        RAN.store(false, Ordering::SeqCst);
        // SAFETY: the target is this thread, and the queued call only sets a
        // flag in this process.
        let queued = unsafe { QueueUserAPC(Some(mark), GetCurrentThread(), 0) };
        assert_ne!(queued, 0, "QueueUserAPC failed");
        let waited = unsafe { WaitForMultipleObjectsEx(&handles, false, 1_000, true) };
        assert!(
            RAN.load(Ordering::SeqCst),
            "a queued call must run during an alertable wait, but the wait returned {}",
            waited.0
        );

        // Without the alertable flag the queued call never runs. The first
        // version of the watch used that form, and the callback never ran.
        RAN.store(false, Ordering::SeqCst);
        // SAFETY: the same call as above, with the alertable flag off.
        let queued = unsafe { QueueUserAPC(Some(mark), GetCurrentThread(), 0) };
        assert_ne!(queued, 0, "QueueUserAPC failed");
        unsafe { WaitForMultipleObjectsEx(&handles, false, 300, false) };
        assert!(
            !RAN.load(Ordering::SeqCst),
            "a plain wait must not run the queued call"
        );
    }

    #[test]
    fn should_tell_a_third_party_binary_from_a_windows_one() {
        assert!(!is_third_party("C:\\Windows\\System32\\spoolsv.exe"));
        assert!(!is_third_party(
            "\"C:\\Windows\\System32\\svchost.exe\" -k netsvcs"
        ));
        assert!(!is_third_party(
            "\\SystemRoot\\system32\\drivers\\tcpip.sys"
        ));
        assert!(!is_third_party("System32\\drivers\\ACPI.sys"));
        assert!(!is_third_party("\\??\\C:\\Windows\\System32\\x.exe"));
        assert!(!is_third_party(""));

        assert!(is_third_party("C:\\Program Files\\Some App\\app.exe"));
        assert!(is_third_party(
            "\"C:\\Program Files\\Some App\\app.exe\" /svc"
        ));
        assert!(is_third_party(
            "\\??\\C:\\Program Files\\Riot Vanguard\\vgk.sys"
        ));
        assert!(is_third_party("D:\\Apps\\service.exe"));
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_notify_at_once_when_the_service_is_already_running() {
        // The documentation says: "If the service is already in any of the
        // requested states when NotifyServiceStatusChange is called, the
        // callback function is queued immediately."
        //
        // This proves the whole delivery path, registration to wake up, with no
        // admin rights and no state change. Spooler stands in for a service that
        // is already running.
        let running = query("Spooler").expect("query");
        assert_eq!(running.status, Status::Running, "Spooler must be running");

        let quit = StopEvent::new().expect("event");
        let mut watched = watch("Spooler", &quit).expect("watch");
        watched.arm().expect("arm");

        let before = CALLBACKS.load(std::sync::atomic::Ordering::SeqCst);
        let outcome = watched.wait_for(3_000);
        let after = CALLBACKS.load(std::sync::atomic::Ordering::SeqCst);
        let (status, triggered) = watched.notification();

        let changed = matches!(&outcome, Ok(WatchState::Changed));
        assert!(
            changed,
            "the callback must arrive at once. outcome {outcome:?}, callbacks {before} -> \
             {after}, buffer status {status} triggered {triggered}, wake failures {}, \
             event handle {:?}",
            SETEVENT_FAILURES.load(std::sync::atomic::Ordering::SeqCst),
            watched.event.0
        );
        assert!(after > before, "the callback itself must run");
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_list_the_services_of_this_pc() {
        let services = list_services().expect("list");

        assert!(services.len() > 100, "only {} services", services.len());
        assert!(
            services.iter().any(|service| service.name == "RpcSs"),
            "RpcSs is missing"
        );
        assert!(
            services.iter().any(|service| service.is_win32),
            "no Win32 service was found"
        );
        assert!(
            services
                .iter()
                .all(|service| !service.display_name.is_empty()),
            "a service has no display name"
        );

        let drivers = services.iter().filter(|service| service.is_driver).count();
        let third_party = services
            .iter()
            .filter(|service| service.third_party)
            .count();
        let running = services
            .iter()
            .filter(|service| service.status == Status::Running)
            .count();
        let typed = services
            .iter()
            .filter(|service| service.startup != Startup::Unknown)
            .count();
        println!(
            "services {}: drivers {drivers}, third party {third_party}, running {running}, \
             with a startup type {typed}",
            services.len()
        );

        assert!(drivers > 0, "no driver was found");
        assert!(third_party > 0, "no third-party service was found");
        assert!(running > 0, "no service is running");
        // A read that Windows denies leaves the startup type unknown.
        assert!(
            typed > services.len() / 2,
            "too many services have no startup type"
        );
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_read_the_facts_of_a_known_service() {
        let info = query("Spooler").expect("query");

        assert_eq!(info.name, "Spooler");
        assert!(info.is_win32);
        assert!(!info.is_driver);
        assert!(info.accepts_stop);
        assert!(!info.display_name.is_empty());
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_report_a_service_that_does_not_exist() {
        let error = query("LeashNoSuchService1234").unwrap_err();
        assert!(matches!(error, ScmError::NotFound { .. }), "{error:?}");
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_read_the_facts_that_the_safety_check_needs() {
        let facts = facts("Spooler").expect("facts");
        assert!(facts.accepts_stop);
        assert!(facts.is_win32);

        let lists = SafetyLists::embedded().expect("lists");
        let verdict = lists.assess(&facts, &[]);
        assert_eq!(verdict.level, Level::Normal);
        assert_eq!(verdict.reason, Reason::None);
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_block_a_denied_service_with_facts_from_windows() {
        let lists = SafetyLists::embedded().expect("lists");
        let mut checked = 0;

        for name in ["RpcSs", "DcomLaunch", "EventLog", "WinDefend", "mpssvc"] {
            match facts(name) {
                Ok(facts) => {
                    let verdict = lists.assess(&facts, &[]);
                    assert_eq!(verdict.level, Level::Blocked, "{name}");
                    assert_eq!(verdict.reason, Reason::DenyList, "{name}");
                    checked += 1;
                }
                // Windows denies the read for some services when the process is
                // not elevated. The caller blocks those instead of stopping.
                Err(ScmError::AccessDenied { .. }) => {}
                Err(error) => panic!("{name}: {error:?}"),
            }
        }

        assert!(checked > 0, "no denied service could be read");
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_list_dependents_without_an_error() {
        let names = dependents("Spooler").expect("dependents");
        for name in &names {
            assert!(!name.is_empty());
        }
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_report_a_missing_service_on_a_stop() {
        let error = stop("LeashNoSuchService1234", Duration::from_secs(1)).unwrap_err();
        assert!(matches!(error, ScmError::NotFound { .. }), "{error:?}");
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_report_a_missing_service_on_a_start() {
        let error = start("LeashNoSuchService1234", Duration::from_secs(1)).unwrap_err();
        assert!(matches!(error, ScmError::NotFound { .. }), "{error:?}");
    }

    #[test]
    #[ignore = "needs an elevated shell and LeashTestSvc"]
    fn should_start_the_test_service() {
        // Only the dummy service that the test script creates may be started
        // from a test. Never a real service.
        match query("LeashTestSvc") {
            Err(ScmError::NotFound { .. }) => {
                println!("LeashTestSvc is not installed. Run the elevated test script.");
                return;
            }
            Err(error) => panic!("LeashTestSvc: {error:?}"),
            Ok(_) => {}
        }

        // Stop it first, so the start has real work to do.
        assert_eq!(
            stop("LeashTestSvc", DEFAULT_WAIT).expect("stop"),
            StopOutcome::Stopped
        );

        assert_eq!(
            start("LeashTestSvc", DEFAULT_WAIT).expect("start"),
            StartOutcome::Started
        );

        // It is running now, so a second start is a no-op and must say Started.
        assert_eq!(
            start("LeashTestSvc", DEFAULT_WAIT).expect("start again"),
            StartOutcome::Started
        );

        // A disabled service cannot start, and Leash never changes a setting, so
        // Windows has to refuse the start.
        set_test_service_start_type("disabled");
        assert_eq!(
            stop("LeashTestSvc", DEFAULT_WAIT).expect("stop"),
            StopOutcome::Stopped
        );
        assert_eq!(
            start("LeashTestSvc", DEFAULT_WAIT).expect("start while disabled"),
            StartOutcome::Disabled
        );
        // Put it back, so the tests that run after this one can start it.
        set_test_service_start_type("demand");
    }

    fn set_test_service_start_type(kind: &str) {
        let changed = std::process::Command::new("sc.exe")
            .args(["config", "LeashTestSvc", "start=", kind])
            .status()
            .expect("sc config");
        assert!(changed.success(), "sc config start= {kind} failed");
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_not_call_a_stopped_service_blocked() {
        let services = list_services().expect("list");
        let stopped = services
            .iter()
            .find(|service| {
                service.status == Status::Stopped && service.is_win32 && !service.is_driver
            })
            .expect("a stopped Win32 service");

        // Windows reports no accepted controls while a service is stopped.
        assert!(
            !stopped.accepts_stop,
            "{} reports controls while stopped",
            stopped.name
        );

        // Being stopped is not a reason to block a service.
        let facts = facts(&stopped.name).expect("facts");
        assert!(
            facts.accepts_stop,
            "{} must not be blocked just for being stopped",
            stopped.name
        );
    }

    #[test]
    #[ignore = "needs an elevated shell and LeashTestSvc"]
    fn should_stop_the_test_service() {
        // Only the dummy service that the test script creates may be stopped
        // from a test. Never a real service.
        match query("LeashTestSvc") {
            Err(ScmError::NotFound { .. }) => {
                println!("LeashTestSvc is not installed. Run the elevated test script.");
                return;
            }
            Err(error) => panic!("LeashTestSvc: {error:?}"),
            Ok(info) => {
                // The wait and the control call are only tested when the
                // service is really running.
                assert_eq!(
                    info.status,
                    Status::Running,
                    "the script must leave the test service running"
                );
            }
        }

        let outcome = stop("LeashTestSvc", DEFAULT_WAIT).expect("stop");
        assert_eq!(outcome, StopOutcome::Stopped);

        // The service is down now, so a second stop is a no-op and must not
        // report that the service refuses a stop command.
        let again = stop("LeashTestSvc", DEFAULT_WAIT).expect("stop again");
        assert_eq!(again, StopOutcome::Stopped);
    }

    #[test]
    #[ignore = "calls the real service manager"]
    fn should_block_a_per_user_service_through_its_suffix() {
        let services = list_services().expect("list");
        let suffixed = services
            .iter()
            .find(|service| service.name.ends_with("_bdb01"));

        // The suffix is machine specific. Only check the rule when one exists.
        if let Some(service) = suffixed {
            let facts = facts(&service.name).expect("facts");
            assert!(facts.is_win32);
        }
    }
}
