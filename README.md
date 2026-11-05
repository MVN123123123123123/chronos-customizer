# Chronos: Display, Lock Screen & Wallpaper Customization Suite

Chronos is an ultra-high performance, battery-efficient Android personalization suite written primarily in **Rust**. It comprises a pure Rust native management application (Slint UI), a companion root module compatible with **Magisk**, **KernelSU (KSU)**, and **APatch**, a systemless Zygisk SystemUI interceptor, and a hardware-accelerated rendering engine.

---

## Key Features

1. **Display Preview (Ambient AOD)**
   - True OLED `#000000` subpixel shutoff.
   - Dynamic LTPO refresh rate negotiation (1Hz - 30Hz) via `ANativeWindow_setFrameRate`.
   - Anti-burn-in circular orbit pixel shifter ($\pm 1$ to $\pm 3$ px every 3 minutes).
   - Event-driven clock tick (pipeline sleeps between minute/second increments).
   - Dynamic OEM AOD auto-suppression matrix for Samsung OneUI, Xiaomi HyperOS, Google Pixel, and OnePlus/Oppo ColorOS with safe in-memory rollback.

2. **Lock Screen Customization (Companion Module)**
   - Systemless SystemUI interceptor (`com.android.systemui`).
   - Intercepts `KeyguardViewMediator` / `StatusBarKeyguardViewManager` while preserving `KeyguardUpdateMonitor`.
   - Biometric authentication glow animations (Particle Ripple, Cyberpunk Pulse, Solar Flare).
   - In-display fingerprint (UDFPS) and Face Unlock status triggers with floating retry button.
   - **Zero-Lockout Safety Guarantee**: Instant automatic fallback to stock Keyguard PIN/Pattern/Password entry upon swipe or biometric failure.

3. **Wallpaper & Live Canvas Engine**
   - **AI Depth Effect (Clock Behind Subject)**: 2.5D layer occlusion with gyroscope parallax ($\Delta X = \pm k \cdot \theta_{\text{yaw}}$).
   - **Hardware Video Wallpaper**: Direct zero-copy `AMediaCodec` hardware decoding into `AHardwareBuffer` / `ANativeWindow`.
   - **Launcher Multi-Page Parallax**: Smooth horizontal viewport shift synchronized with home screen swipe offsets (`onOffsetsChanged`).
   - **Cross-State Reactive Transitions**: Progressive Gaussian blur (0 to 24px) when unlocking from Lock Screen to Home Screen.

4. **100% Systemless & Locked Bootloader Safe**
   - Strictly memory-backed (`tmpfs`) overlays and process hooks.
   - Zero physical writes to `/system` or `/boot` (AVB2.0 / dm-verity compliant).
   - Guaranteed clean rollback upon device reboot.

5. **Android 10 - 17 Compatibility & 16KB Page Alignment**
   - Compiled with `-Wl,-z,max-page-size=16384` for Android 15-17 16KB ELF page size alignment.
   - Automated Doze mode and AppOps whitelisting (`dumpsys deviceidle whitelist`).

---

## Crate Layout

- [`chronos-common`](chronos-common/): Shared configuration models, state machine transitions, and Unix domain socket IPC framing.
- [`chronos-engine`](chronos-engine/): Vulkan/GLES shaders, surface bindings, AMediaCodec video player, 2.5D depth occlusion, and modular widgets.
- [`chronos-daemon`](chronos-daemon/): Root companion binary (`customd`) providing Doze whitelisting, OEM AOD suppression, and root overlay fallback.
- [`chronos-zygisk`](chronos-zygisk/): Native Zygisk SystemUI hook for lock screen interception and biometric callbacks.
- [`chronos-app`](chronos-app/): Native Slint dark-themed OLED UI manager application and Android service declarations.
- [`chronos-module`](chronos-module/): Unified Magisk / KernelSU / APatch flashable companion module tree.

---

## Build & Test Instructions

### 1. Run Unit & Integration Tests (Host)
```bash
make test
```

### 2. Build Release Binaries for Host
```bash
make build-host
```

### 3. Cross-Compile for Android (ARM64 with 16KB Page Size)
```bash
make build-android
```

### 4. Package Flashable Companion Module Zip
```bash
make package-module
# Output: dist/chronos-companion-module.zip
```

### 5. Verify 16KB ELF Alignment
```bash
make verify-16kb
```
