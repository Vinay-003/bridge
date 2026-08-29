package com.bridge.android.service

import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import android.app.Notification
import org.json.JSONObject

class BridgeNotificationListener : NotificationListenerService() {
    override fun onNotificationPosted(sbn: StatusBarNotification) {
        val n = sbn.notification
        val title = n.extras.getCharSequence(Notification.EXTRA_TITLE)?.toString() ?: ""
        val body = n.extras.getCharSequence(Notification.EXTRA_TEXT)?.toString() ?: ""
        val pkg = sbn.packageName
        // filter + send notify.new via BridgeService WS — stub for MVP
        // BridgeService.send(NotifyNew{key=sbn.key, app=pkg, title, body, hasReply=...})
    }
    override fun onNotificationRemoved(sbn: StatusBarNotification) {
        // send dismiss sync
    }
}
