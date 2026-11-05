package dev.chronos.customizer;

import android.service.wallpaper.WallpaperService;
import android.view.SurfaceHolder;

public class ChronosWallpaperService extends WallpaperService {
    static {
        System.loadLibrary("chronos_app");
    }

    private static native void nativeOnSurfaceCreated(int width, int height);
    private static native void nativeOnOffsetsChanged(float xOffset, float yOffset);

    @Override
    public Engine onCreateEngine() {
        return new ChronosEngine();
    }

    private class ChronosEngine extends Engine {
        @Override
        public void onSurfaceCreated(SurfaceHolder holder) {
            super.onSurfaceCreated(holder);
            android.graphics.Rect frame = holder.getSurfaceFrame();
            nativeOnSurfaceCreated(frame.width(), frame.height());
        }

        @Override
        public void onOffsetsChanged(float xOffset, float yOffset, float xOffsetStep, float yOffsetStep, int xPixelOffset, int yPixelOffset) {
            super.onOffsetsChanged(xOffset, yOffset, xOffsetStep, yOffsetStep, xPixelOffset, yPixelOffset);
            nativeOnOffsetsChanged(xOffset, yOffset);
        }
    }
}
