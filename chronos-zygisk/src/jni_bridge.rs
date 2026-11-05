//! Safe JNI Helpers for SystemUI Keyguard & Biometric Hooking

use jni::objects::{JClass, JObject, JString, JValue};
use jni::JNIEnv;

pub struct JniHelper<'a> {
    pub env: JNIEnv<'a>,
}

impl<'a> JniHelper<'a> {
    pub fn new(env: JNIEnv<'a>) -> Self {
        Self { env }
    }

    pub fn set_view_visibility(
        &mut self,
        view: &JObject,
        visibility: i32,
    ) -> Result<(), jni::errors::Error> {
        self.env
            .call_method(view, "setVisibility", "(I)V", &[JValue::Int(visibility)])?;
        Ok(())
    }

    pub fn find_systemui_class(
        &mut self,
        class_name: &str,
    ) -> Result<JClass<'a>, jni::errors::Error> {
        self.env.find_class(class_name)
    }

    pub fn log_java_string(&mut self, jstr: &JString) {
        if let Ok(rust_str) = self.env.get_string(jstr) {
            log::info!("SystemUI JNI String: {}", rust_str.to_string_lossy());
        }
    }
}
