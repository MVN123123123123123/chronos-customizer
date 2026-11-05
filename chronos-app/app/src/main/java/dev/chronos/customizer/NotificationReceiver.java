package dev.chronos.customizer;

import android.service.notification.NotificationListenerService;
import android.service.notification.StatusBarNotification;

public class NotificationReceiver extends NotificationListenerService {
    static {
        System.loadLibrary("chronos_app");
    }

    private static native void nativeOnNotificationPosted(String packageName, int count);
    private static native void nativeOnNotificationRemoved(String packageName);

    @Override
    public void onNotificationPosted(StatusBarNotification sbn) {
        if (sbn != null && sbn.getPackageName() != null) {
            nativeOnNotificationPosted(sbn.getPackageName(), 1);
        }
    }

    @Override
    public void onNotificationRemoved(StatusBarNotification sbn) {
        if (sbn != null && sbn.getPackageName() != null) {
            nativeOnNotificationRemoved(sbn.getPackageName());
        }
    }
}
