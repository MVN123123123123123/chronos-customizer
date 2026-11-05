//! Hardware-accelerated AMediaCodec video player with zero CPU memory copies

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VideoError {
    #[error("Failed to open video source: {0}")]
    OpenFailed(String),
    #[error("Unsupported video format or codec: {0}")]
    UnsupportedCodec(String),
    #[error("Decoder initialization error: {0}")]
    DecoderInitError(String),
    #[error("Playback error: {0}")]
    PlaybackError(String),
}

pub trait VideoDecoder: Send + Sync {
    fn open(&mut self, path: &str) -> Result<(), VideoError>;
    fn open_with_surface(
        &mut self,
        path: &str,
        _surface_ptr: *mut std::ffi::c_void,
    ) -> Result<(), VideoError> {
        self.open(path)
    }
    fn step_frame(&mut self) -> Result<bool, VideoError>;
    fn seek(&mut self, timestamp_us: u64) -> Result<(), VideoError>;
    fn duration_us(&self) -> u64;
    fn video_dimensions(&self) -> (u32, u32);
}

/// Mock decoder for unit tests and non-Android environments
#[derive(Debug, Default)]
pub struct MockVideoDecoder {
    path: String,
    current_time_us: u64,
    duration_us: u64,
    width: u32,
    height: u32,
    is_playing: bool,
    has_surface: bool,
}

impl VideoDecoder for MockVideoDecoder {
    fn open(&mut self, path: &str) -> Result<(), VideoError> {
        self.path = path.to_string();
        self.duration_us = 10_000_000; // 10s mock
        self.width = 1080;
        self.height = 1920;
        self.current_time_us = 0;
        self.is_playing = true;
        Ok(())
    }

    fn open_with_surface(
        &mut self,
        path: &str,
        surface_ptr: *mut std::ffi::c_void,
    ) -> Result<(), VideoError> {
        self.has_surface = !surface_ptr.is_null();
        self.open(path)
    }

    fn step_frame(&mut self) -> Result<bool, VideoError> {
        if !self.is_playing {
            return Ok(false);
        }
        self.current_time_us += 16_666; // 60fps step
        if self.current_time_us >= self.duration_us {
            self.current_time_us = 0; // Loop playback
        }
        Ok(true)
    }

    fn seek(&mut self, timestamp_us: u64) -> Result<(), VideoError> {
        self.current_time_us = timestamp_us.min(self.duration_us);
        Ok(())
    }

    fn duration_us(&self) -> u64 {
        self.duration_us
    }

    fn video_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[cfg(target_os = "android")]
pub mod android {
    use super::*;
    use std::ffi::CString;

    #[derive(Default)]
    pub struct AndroidMediaCodecDecoder {
        extractor: *mut ndk_sys::AMediaExtractor,
        codec: *mut ndk_sys::AMediaCodec,
        duration_us: u64,
        width: u32,
        height: u32,
        native_window: *mut ndk_sys::ANativeWindow,
    }

    unsafe impl Send for AndroidMediaCodecDecoder {}
    unsafe impl Sync for AndroidMediaCodecDecoder {}

    impl AndroidMediaCodecDecoder {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl VideoDecoder for AndroidMediaCodecDecoder {
        fn open(&mut self, path: &str) -> Result<(), VideoError> {
            self.open_with_surface(path, std::ptr::null_mut())
        }

        fn open_with_surface(
            &mut self,
            path: &str,
            surface_ptr: *mut std::ffi::c_void,
        ) -> Result<(), VideoError> {
            self.native_window = surface_ptr as *mut ndk_sys::ANativeWindow;
            unsafe {
                self.extractor = ndk_sys::AMediaExtractor_new();
                if self.extractor.is_null() {
                    return Err(VideoError::OpenFailed(
                        "Cannot allocate AMediaExtractor".into(),
                    ));
                }

                let c_path =
                    CString::new(path).map_err(|e| VideoError::OpenFailed(e.to_string()))?;
                let status =
                    ndk_sys::AMediaExtractor_setDataSource(self.extractor, c_path.as_ptr());
                if status != ndk_sys::media_status_t::AMEDIA_OK {
                    return Err(VideoError::OpenFailed(format!(
                        "setDataSource failed with code {:?}",
                        status
                    )));
                }

                let num_tracks = ndk_sys::AMediaExtractor_getTrackCount(self.extractor);
                for i in 0..num_tracks {
                    let format = ndk_sys::AMediaExtractor_getTrackFormat(self.extractor, i);
                    let mut mime_ptr: *const std::os::raw::c_char = std::ptr::null();
                    if ndk_sys::AMediaFormat_getString(
                        format,
                        ndk_sys::AMEDIAFORMAT_KEY_MIME,
                        &mut mime_ptr,
                    ) {
                        let c_mime = std::ffi::CStr::from_ptr(mime_ptr).to_string_lossy();
                        if c_mime.starts_with("video/") {
                            ndk_sys::AMediaExtractor_selectTrack(self.extractor, i);
                            self.codec = ndk_sys::AMediaCodec_createDecoderByType(mime_ptr);
                            // Zero-copy direct hardware surface configuration
                            ndk_sys::AMediaCodec_configure(
                                self.codec,
                                format,
                                self.native_window,
                                std::ptr::null_mut(),
                                0,
                            );
                            ndk_sys::AMediaCodec_start(self.codec);
                            ndk_sys::AMediaFormat_delete(format);
                            break;
                        }
                    }
                    ndk_sys::AMediaFormat_delete(format);
                }
            }
            Ok(())
        }

        fn step_frame(&mut self) -> Result<bool, VideoError> {
            if self.codec.is_null() {
                return Ok(false);
            }
            unsafe {
                // Read from extractor, feed to codec, render to surface with zero copy
                let in_index = ndk_sys::AMediaCodec_dequeueInputBuffer(self.codec, 2000);
                if in_index >= 0 {
                    let mut buf_size: usize = 0;
                    let buf_ptr = ndk_sys::AMediaCodec_getInputBuffer(
                        self.codec,
                        in_index as usize,
                        &mut buf_size,
                    );
                    if !buf_ptr.is_null() {
                        let sample_size = ndk_sys::AMediaExtractor_readSampleData(
                            self.extractor,
                            buf_ptr,
                            buf_size,
                        );
                        if sample_size < 0 {
                            // End of stream, seek back to beginning for seamless loop
                            ndk_sys::AMediaExtractor_seekTo(
                                self.extractor,
                                0,
                                ndk_sys::SeekMode::AMEDIAEXTRACTOR_SEEK_CLOSEST_SYNC,
                            );
                        } else {
                            let pts = ndk_sys::AMediaExtractor_getSampleTime(self.extractor);
                            ndk_sys::AMediaCodec_queueInputBuffer(
                                self.codec,
                                in_index as usize,
                                0,
                                sample_size as usize,
                                pts as u64,
                                0,
                            );
                            ndk_sys::AMediaExtractor_advance(self.extractor);
                        }
                    }
                }

                let mut info = ndk_sys::AMediaCodecBufferInfo {
                    offset: 0,
                    size: 0,
                    presentationTimeUs: 0,
                    flags: 0,
                };
                let out_index =
                    ndk_sys::AMediaCodec_dequeueOutputBuffer(self.codec, &mut info, 2000);
                if out_index >= 0 {
                    // Render directly to ANativeWindow / AHB (render = true)
                    ndk_sys::AMediaCodec_releaseOutputBuffer(self.codec, out_index as usize, true);
                    return Ok(true);
                }
            }
            Ok(false)
        }

        fn seek(&mut self, timestamp_us: u64) -> Result<(), VideoError> {
            if !self.extractor.is_null() {
                unsafe {
                    ndk_sys::AMediaExtractor_seekTo(
                        self.extractor,
                        timestamp_us as i64,
                        ndk_sys::SeekMode::AMEDIAEXTRACTOR_SEEK_CLOSEST_SYNC,
                    );
                }
            }
            Ok(())
        }

        fn duration_us(&self) -> u64 {
            self.duration_us
        }

        fn video_dimensions(&self) -> (u32, u32) {
            (self.width, self.height)
        }
    }

    impl Drop for AndroidMediaCodecDecoder {
        fn drop(&mut self) {
            unsafe {
                if !self.codec.is_null() {
                    ndk_sys::AMediaCodec_stop(self.codec);
                    ndk_sys::AMediaCodec_delete(self.codec);
                }
                if !self.extractor.is_null() {
                    ndk_sys::AMediaExtractor_delete(self.extractor);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_video_decoder() {
        let mut decoder = MockVideoDecoder::default();
        decoder.open("/sdcard/wallpaper.mp4").expect("open failed");
        assert_eq!(decoder.duration_us(), 10_000_000);
        assert_eq!(decoder.video_dimensions(), (1080, 1920));
        assert!(decoder.step_frame().expect("step failed"));

        let dummy_surface = 0x1234 as *mut std::ffi::c_void;
        decoder
            .open_with_surface("/sdcard/wallpaper.mp4", dummy_surface)
            .expect("open with surface failed");
        assert!(decoder.has_surface);
    }
}
