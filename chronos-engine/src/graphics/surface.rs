//! Surface abstraction for ANativeWindow and AHardwareBuffer

use std::fmt::Debug;

pub trait RenderSurface: Debug + Send + Sync {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn set_frame_rate(&self, fps: f32) -> Result<(), SurfaceError>;
    fn swap_buffers(&mut self) -> Result<(), SurfaceError>;
}

#[derive(Debug, thiserror::Error)]
pub enum SurfaceError {
    #[error("Null surface pointer")]
    NullSurface,
    #[error("Failed to acquire hardware buffer")]
    BufferAcquisitionFailed,
    #[error("Surface configuration error: {0}")]
    ConfigError(String),
}

/// Headless / Mock surface used for testing and host compilation
#[derive(Debug)]
pub struct MockSurface {
    width: u32,
    height: u32,
    pub current_fps: f32,
    pub swap_count: u64,
}

impl MockSurface {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            current_fps: 60.0,
            swap_count: 0,
        }
    }
}

impl RenderSurface for MockSurface {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn set_frame_rate(&self, fps: f32) -> Result<(), SurfaceError> {
        // Mock set frame rate for LTPO panels (e.g. 1Hz)
        log::debug!("Setting surface frame rate to {} Hz", fps);
        Ok(())
    }

    fn swap_buffers(&mut self) -> Result<(), SurfaceError> {
        self.swap_count += 1;
        Ok(())
    }
}

#[cfg(target_os = "android")]
pub mod android {
    use super::*;
    use ndk::native_window::NativeWindow;

    #[derive(Debug)]
    pub struct AndroidNativeSurface {
        window: NativeWindow,
    }

    impl AndroidNativeSurface {
        pub fn from_native_window(window: NativeWindow) -> Self {
            Self { window }
        }
    }

    impl RenderSurface for AndroidNativeSurface {
        fn width(&self) -> u32 {
            self.window.width() as u32
        }

        fn height(&self) -> u32 {
            self.window.height() as u32
        }

        fn set_frame_rate(&self, fps: f32) -> Result<(), SurfaceError> {
            // In NDK API 30+, ANativeWindow_setFrameRate adjusts refresh rate
            unsafe {
                let raw_ptr = self.window.ptr().as_ptr();
                if !raw_ptr.is_null() {
                    ndk_sys::ANativeWindow_setFrameRate(raw_ptr, fps, 0i8);
                }
            }
            Ok(())
        }

        fn swap_buffers(&mut self) -> Result<(), SurfaceError> {
            // EGL or Vulkan presentation swaps
            Ok(())
        }
    }
}
