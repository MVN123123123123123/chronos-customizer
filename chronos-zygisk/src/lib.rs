//! Zygisk Native Module for SystemUI Keyguard Customization

pub mod hooks;
pub mod jni_bridge;

use hooks::{BiometricHookEngine, KeyguardHookEngine};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub static KEYGUARD_HOOK: Mutex<Option<KeyguardHookEngine>> = Mutex::new(None);
pub static BIOMETRIC_HOOK: Mutex<Option<BiometricHookEngine>> = Mutex::new(None);
pub static IS_SYSTEMUI: AtomicBool = AtomicBool::new(false);

#[repr(C)]
pub struct ZygiskApi {
    pub version: usize,
    pub impl_ptr: *mut std::ffi::c_void,
    pub register_module:
        Option<unsafe extern "C" fn(api: *mut ZygiskApi, module: *mut ZygiskModuleAbi) -> bool>,
    pub hook_jni_native_methods: Option<
        unsafe extern "C" fn(
            env: *mut jni::sys::JNIEnv,
            class_name: *const std::os::raw::c_char,
            methods: *const std::ffi::c_void,
            count: i32,
        ),
    >,
    pub plt_hook_register: Option<
        unsafe extern "C" fn(
            regex: *const std::os::raw::c_char,
            symbol: *const std::os::raw::c_char,
            new_fn: *mut std::ffi::c_void,
            old_fn: *mut *mut std::ffi::c_void,
        ),
    >,
    pub plt_hook_commit: Option<unsafe extern "C" fn() -> bool>,
    pub connect_companion: Option<unsafe extern "C" fn(api: *mut ZygiskApi) -> i32>,
    pub set_option: Option<unsafe extern "C" fn(api: *mut ZygiskApi, opt: i32)>,
}

#[repr(C)]
pub struct AppSpecializeArgs {
    pub uid: *mut jni::sys::jint,
    pub gid: *mut jni::sys::jint,
    pub gids: *mut jni::sys::jintArray,
    pub runtime_flags: *mut jni::sys::jint,
    pub rlimits: *mut jni::sys::jobjectArray,
    pub mount_external: *mut jni::sys::jint,
    pub se_info: *mut jni::sys::jstring,
    pub nice_name: *mut jni::sys::jstring,
    pub is_child_zygote: *mut jni::sys::jboolean,
    pub instruction_set: *mut jni::sys::jstring,
    pub app_data_dir: *mut jni::sys::jstring,
}

#[repr(C)]
pub struct ServerSpecializeArgs {
    pub uid: *mut jni::sys::jint,
    pub gid: *mut jni::sys::jint,
    pub gids: *mut jni::sys::jintArray,
    pub runtime_flags: *mut jni::sys::jint,
    pub rlimits: *mut jni::sys::jobjectArray,
    pub permitted_capabilities: *mut jni::sys::jlong,
    pub effective_capabilities: *mut jni::sys::jlong,
}

#[repr(C)]
pub struct ZygiskModuleAbi {
    pub api_version: usize,
    pub impl_ptr: *mut std::ffi::c_void,
    pub pre_app_specialize:
        Option<unsafe extern "C" fn(impl_ptr: *mut std::ffi::c_void, args: *mut AppSpecializeArgs)>,
    pub post_app_specialize: Option<
        unsafe extern "C" fn(impl_ptr: *mut std::ffi::c_void, args: *const AppSpecializeArgs),
    >,
    pub pre_server_specialize: Option<
        unsafe extern "C" fn(impl_ptr: *mut std::ffi::c_void, args: *mut ServerSpecializeArgs),
    >,
    pub post_server_specialize: Option<
        unsafe extern "C" fn(impl_ptr: *mut std::ffi::c_void, args: *const ServerSpecializeArgs),
    >,
}

/// Zygisk callback invoked prior to app specialization.
///
/// # Safety
///
/// The caller must ensure that pointers passed (or null pointers) conform to the Zygisk ABI contract.
pub unsafe extern "C" fn zygisk_pre_app_specialize_cb(
    _impl_ptr: *mut std::ffi::c_void,
    _args: *mut AppSpecializeArgs,
) {
    if let Ok(cmdline) = std::fs::read_to_string("/proc/self/cmdline") {
        if cmdline.starts_with("com.android.systemui") {
            IS_SYSTEMUI.store(true, Ordering::SeqCst);
            log::info!("Chronos Zygisk: Identified target process: com.android.systemui");
        }
    }
}

/// Zygisk callback invoked after app specialization.
///
/// # Safety
///
/// The caller must ensure that pointers passed (or null pointers) conform to the Zygisk ABI contract.
pub unsafe extern "C" fn zygisk_post_app_specialize_cb(
    _impl_ptr: *mut std::ffi::c_void,
    _args: *const AppSpecializeArgs,
) {
    if IS_SYSTEMUI.load(Ordering::SeqCst) {
        log::info!("Chronos Zygisk: Specialized inside com.android.systemui, installing Keyguard & Biometric interceptors");
        if let Some(ref mut kg) = *KEYGUARD_HOOK.lock().unwrap() {
            kg.hide_stock_lockscreen();
        }
    }
}

pub fn init_zygisk_module() {
    log::info!("Initializing Chronos Zygisk Module for SystemUI...");
    let mut kg = KEYGUARD_HOOK.lock().unwrap();
    *kg = Some(KeyguardHookEngine::new());
    let mut bio = BIOMETRIC_HOOK.lock().unwrap();
    *bio = Some(BiometricHookEngine::new());
}

/// Official Zygisk Module Entry Point for Magisk, KernelSU, and APatch
///
/// # Safety
///
/// `api` must be a valid pointer to a `ZygiskApi` structure or null, and `_env` must be a valid
/// pointer to a JNIEnv or null, as provided by the Zygisk runtime.
#[no_mangle]
pub unsafe extern "C" fn zygisk_module_entry(api: *mut ZygiskApi, _env: *mut jni::sys::JNIEnv) {
    init_zygisk_module();
    if !api.is_null() {
        if let Some(register) = (*api).register_module {
            static mut MODULE_ABI: ZygiskModuleAbi = ZygiskModuleAbi {
                api_version: 4,
                impl_ptr: std::ptr::null_mut(),
                pre_app_specialize: Some(zygisk_pre_app_specialize_cb),
                post_app_specialize: Some(zygisk_post_app_specialize_cb),
                pre_server_specialize: None,
                post_server_specialize: None,
            };
            register(api, &raw mut MODULE_ABI);
            log::info!(
                "Chronos Zygisk: Successfully registered module ABI with Zygisk v{}",
                (*api).version
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static mut MOCK_REGISTERED: bool = false;

    unsafe extern "C" fn mock_register_module(
        _api: *mut ZygiskApi,
        module: *mut ZygiskModuleAbi,
    ) -> bool {
        assert!(!module.is_null());
        assert_eq!((*module).api_version, 4);
        assert!((*module).pre_app_specialize.is_some());
        assert!((*module).post_app_specialize.is_some());
        MOCK_REGISTERED = true;
        true
    }

    #[test]
    fn test_zygisk_module_registration() {
        let mut mock_api = ZygiskApi {
            version: 4,
            impl_ptr: std::ptr::null_mut(),
            register_module: Some(mock_register_module),
            hook_jni_native_methods: None,
            plt_hook_register: None,
            plt_hook_commit: None,
            connect_companion: None,
            set_option: None,
        };

        unsafe {
            zygisk_module_entry(&mut mock_api, std::ptr::null_mut());
            assert!(MOCK_REGISTERED);
        }

        let kg = KEYGUARD_HOOK.lock().unwrap();
        assert!(kg.is_some());
    }

    #[test]
    fn test_post_app_specialize_trigger() {
        init_zygisk_module();
        IS_SYSTEMUI.store(true, Ordering::SeqCst);
        unsafe {
            zygisk_post_app_specialize_cb(std::ptr::null_mut(), std::ptr::null());
        }

        let kg = KEYGUARD_HOOK.lock().unwrap();
        assert!(kg.as_ref().unwrap().is_stock_keyguard_hidden);
    }
}
