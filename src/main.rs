mod click;
mod control;
mod learned;
mod tracking;
use std::ffi::{c_char, CStr, CString};
extern "C" {
    fn gaze_run();
    fn gaze_vision_probe() -> bool;
}
fn main() {
    if std::env::args().any(|a| a == "--vision-probe") {
        if !unsafe { gaze_vision_probe() } {
            eprintln!("Apple Vision probe failed");
            std::process::exit(1);
        }
        println!("Apple Vision hand + face requests passed on a blank local frame.");
        return;
    }
    unsafe {
        gaze_run();
    }
}
#[no_mangle]
pub extern "C" fn gaze_create() -> *mut control::Engine {
    Box::into_raw(Box::new(control::Engine::default()))
}
#[no_mangle]
/// # Safety
/// tracker must be a live, uniquely borrowed gaze_create allocation; json must
/// remain valid UTF-8 throughout this call. Free the result with gaze_free.
pub unsafe extern "C" fn gaze_process(
    tracker: *mut control::Engine,
    json: *const c_char,
) -> *mut c_char {
    if tracker.is_null() || json.is_null() {
        return std::ptr::null_mut();
    }
    let result = CStr::from_ptr(json)
        .to_str()
        .ok()
        .and_then(|s| serde_json::from_str::<control::Input>(s).ok())
        .map(|frame| serde_json::to_value((*tracker).update(frame)).unwrap())
        .unwrap_or_else(|| serde_json::json!({"error":"Neplatný snímek detekce."}));
    CString::new(result.to_string()).unwrap().into_raw()
}
#[no_mangle]
/// # Safety
/// tracker must be a live exclusively borrowed gaze_create allocation.
pub unsafe extern "C" fn gaze_reset(tracker: *mut control::Engine) {
    if let Some(engine) = tracker.as_mut() {
        engine.reset_motion();
    }
}
#[no_mangle]
/// # Safety
/// tracker must be live and exclusively borrowed; json must be a valid C string.
pub unsafe extern "C" fn gaze_command(
    tracker: *mut control::Engine,
    json: *const c_char,
) -> *mut c_char {
    if tracker.is_null() || json.is_null() {
        return std::ptr::null_mut();
    }
    let result = CStr::from_ptr(json)
        .to_str()
        .ok()
        .and_then(|s| serde_json::from_str(s).ok())
        .map(|c| (*tracker).command(c))
        .unwrap_or_else(|| serde_json::json!({"ok":false,"message":"Neplatný příkaz"}));
    CString::new(result.to_string()).unwrap().into_raw()
}
#[no_mangle]
/// # Safety
/// tracker must be null or a live gaze_create allocation not used elsewhere.
pub unsafe extern "C" fn gaze_destroy(tracker: *mut control::Engine) {
    if !tracker.is_null() {
        drop(Box::from_raw(tracker));
    }
}
#[no_mangle]
/// # Safety
/// json must be null or an unfreed gaze_process allocation.
pub unsafe extern "C" fn gaze_free(json: *mut c_char) {
    if !json.is_null() {
        drop(CString::from_raw(json));
    }
}
