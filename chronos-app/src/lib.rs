//! JNI Exports for Android WallpaperService and NotificationListenerService

use jni::objects::{JClass, JString};
use jni::sys::{jfloat, jint};
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_dev_chronos_customizer_ChronosWallpaperService_nativeOnSurfaceCreated(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    log::info!("JNI: Wallpaper surface created: {}x{}", width, height);
}

#[no_mangle]
pub extern "system" fn Java_dev_chronos_customizer_ChronosWallpaperService_nativeOnOffsetsChanged(
    _env: JNIEnv,
    _class: JClass,
    x_offset: jfloat,
    y_offset: jfloat,
) {
    log::trace!(
        "JNI: Wallpaper launcher offset changed: x={}, y={}",
        x_offset,
        y_offset
    );
}

#[no_mangle]
pub extern "system" fn Java_dev_chronos_customizer_NotificationReceiver_nativeOnNotificationPosted(
    mut env: JNIEnv,
    _class: JClass,
    package_name: JString,
    count: jint,
) {
    if let Ok(pkg) = env.get_string(&package_name) {
        log::info!(
            "JNI: Notification posted from: {}, count: {}",
            pkg.to_string_lossy(),
            count
        );
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_chronos_customizer_NotificationReceiver_nativeOnNotificationRemoved(
    mut env: JNIEnv,
    _class: JClass,
    package_name: JString,
) {
    if let Ok(pkg) = env.get_string(&package_name) {
        log::info!("JNI: Notification removed for: {}", pkg.to_string_lossy());
    }
}
