//! Embedded GLSL Shaders for OLED Display Preview, Lock Screen, and Wallpaper Engine

pub const MORPH_CLOCK_VERT: &str = r#"#version 300 es
layout (location = 0) in vec2 aPosition;
layout (location = 1) in vec2 aTexCoord;

uniform vec2 uBurnInOffset;
uniform vec2 uParallaxOffset;
uniform float uScale;
uniform float uMorphProgress;
uniform mat4 uProjection;

out vec2 vTexCoord;

void main() {
    vTexCoord = aTexCoord;
    vec2 pos = aPosition * uScale;
    pos += uBurnInOffset;
    pos += uParallaxOffset;
    gl_Position = uProjection * vec4(pos, 0.0, 1.0);
}
"#;

pub const MORPH_CLOCK_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 vTexCoord;
out vec4 fragColor;

uniform sampler2D uFontTexture;
uniform vec4 uClockColor;
uniform float uGlowIntensity;
uniform float uDimFactor;

void main() {
    float alpha = texture(uFontTexture, vTexCoord).r;
    if (alpha < 0.01) {
        discard; // Save OLED fill-rate & power
    }
    vec3 color = uClockColor.rgb * uDimFactor;
    // Add subtle ambient glow
    color += color * uGlowIntensity * 0.4;
    fragColor = vec4(color, alpha * uClockColor.a);
}
"#;

pub const DEPTH_OCCLUSION_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 vTexCoord;
out vec4 fragColor;

uniform sampler2D uBackgroundTex;
uniform sampler2D uSubjectTex;
uniform sampler2D uClockTex;
uniform sampler2D uDepthMap;

uniform float uDepthThreshold; // Depth cutoff for clock (Z_clock)
uniform float uParallaxBg;
uniform float uParallaxFg;
uniform vec2 uGyroTilt; // (yaw, pitch)

void main() {
    vec2 bgCoord = vTexCoord + uGyroTilt * uParallaxBg;
    vec2 fgCoord = vTexCoord - uGyroTilt * uParallaxFg;

    vec4 bg = texture(uBackgroundTex, bgCoord);
    vec4 clock = texture(uClockTex, vTexCoord);
    vec4 fg = texture(uSubjectTex, fgCoord);
    float depth = texture(uDepthMap, fgCoord).r;

    // 3-pass composite:
    // 1. Base Background
    vec4 color = bg;

    // 2. Clock layer (behind subject if depth > uDepthThreshold)
    color = mix(color, clock, clock.a);

    // 3. Foreground Subject (if subject depth is closer than threshold)
    if (depth <= uDepthThreshold) {
        color = mix(color, fg, fg.a);
    }

    fragColor = color;
}
"#;

pub const BIOMETRIC_RIPPLE_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 vTexCoord;
out vec4 fragColor;

uniform vec2 uCenter;
uniform float uTime;
uniform float uIntensity;
uniform vec3 uScanColor;

void main() {
    vec2 diff = vTexCoord - uCenter;
    float dist = length(diff);

    // Expanding concentric wave
    float wave = sin(dist * 60.0 - uTime * 8.0);
    float ring = smoothstep(0.02, 0.0, abs(dist - mod(uTime * 0.15, 0.25)));

    float alpha = (wave * 0.5 + 0.5) * ring * uIntensity;
    if (alpha < 0.01) {
        discard;
    }
    fragColor = vec4(uScanColor, alpha);
}
"#;

pub const GAUSSIAN_BLUR_FRAG: &str = r#"#version 300 es
precision highp float;

in vec2 vTexCoord;
out vec4 fragColor;

uniform sampler2D uInputTexture;
uniform vec2 uResolution;
uniform float uBlurRadius;

void main() {
    if (uBlurRadius <= 0.1) {
        fragColor = texture(uInputTexture, vTexCoord);
        return;
    }

    vec2 texelSize = 1.0 / uResolution;
    vec4 sum = vec4(0.0);
    float totalWeight = 0.0;

    // 9-tap 1D/2D approximation
    for (float x = -2.0; x <= 2.0; x += 1.0) {
        for (float y = -2.0; y <= 2.0; y += 1.0) {
            float weight = exp(-(x*x + y*y) / (2.0 * uBlurRadius));
            vec2 offset = vec2(x, y) * texelSize * uBlurRadius;
            sum += texture(uInputTexture, vTexCoord + offset) * weight;
            totalWeight += weight;
        }
    }

    fragColor = sum / totalWeight;
}
"#;
