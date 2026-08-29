package com.bridge.android.service

import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import android.app.Notification
import android.app.RemoteInput
import android.content.Intent
import android.os.Bundle

class BridgeNotificationListener : NotificationListenerService() {
    companion object {
        var instance: BridgeNotificationListener? = null
    }
    override fun onCreate() { super.onCreate(); instance = this }
    override fun onDestroy() { instance = null; super.onDestroy() }

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        if (sbn.packageName == packageName) return
        if (sbn.isOngoing) return
        val n = sbn.notification
        val title = n.extras.getCharSequence(Notification.EXTRA_TITLE)?.toString() ?: ""
        val body = n.extras.getCharSequence(Notification.EXTRA_TEXT)?.toString() ?: ""
        if (title.isBlank() && body.isBlank()) return
        val pkg = sbn.packageName
        val hasReply = n.actions?.any { it.remoteInputs != null && it.remoteInputs.isNotEmpty() } ?: false
        val prefs = getSharedPreferences("bridge", 0)
        val host = prefs.getString("host","192.168.1.36") ?: "192.168.1.36"
        val port = prefs.getInt("port",8443)
        val payload = org.json.JSONObject().apply {
            put("key", sbn.key)
            put("app", pkg)
            put("title", title)
            put("body", body)
            put("ts", sbn.postTime)
            put("hasReply", hasReply)
        }
        val msg = org.json.JSONObject().apply {
            put("v",1); put("id", java.util.UUID.randomUUID().toString()); put("type","notify.new"); put("ts", System.currentTimeMillis()); put("nonce","a"); put("payload", payload)
        }
        Thread {
            try {
                val uri = java.net.URI("ws://$host:$port")
                val c = object: org.java_websocket.client.WebSocketClient(uri) {
                    override fun onOpen(h: org.java_websocket.handshake.ServerHandshake?) { send(msg.toString()); close() }
                    override fun onMessage(m: String?) {}
                    override fun onClose(c: Int, r: String?, re: Boolean) {}
                    override fun onError(e: Exception?) {}
                }
                c.connectBlocking()
            } catch(_: Exception) {}
        }.start()
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification) {
        val prefs = getSharedPreferences("bridge", 0)
        val host = prefs.getString("host","192.168.1.36") ?: "192.168.1.36"
        val port = prefs.getInt("port",8443)
        val msg = org.json.JSONObject().apply {
            put("v",1); put("id", java.util.UUID.randomUUID().toString()); put("type","notify.action"); put("ts", System.currentTimeMillis()); put("nonce","a"); put("payload", org.json.JSONObject().apply { put("key", sbn.key); put("action","dismiss") })
        }
        Thread {
            try {
                val uri = java.net.URI("ws://$host:$port")
                val c = object: org.java_websocket.client.WebSocketClient(uri) {
                    override fun onOpen(h: org.java_websocket.handshake.ServerHandshake?) { send(msg.toString()); close() }
                    override fun onMessage(m: String?) {}
                    override fun onClose(c: Int, r: String?, re: Boolean) {}
                    override fun onError(e: Exception?) {}
                }
                c.connectBlocking()
            } catch(_: Exception) {}
        }.start()
    }

    fun replyToNotification(key: String, text: String) {
        try {
            val sbn = activeNotifications.find { it.key == key } ?: return
            val n = sbn.notification
            for (action in n.actions ?: return) {
                val inputs = action.remoteInputs ?: continue
                if (inputs.isEmpty()) continue
                val intent = Intent()
                val bundle = Bundle()
                for (ri in inputs) bundle.putCharSequence(ri.resultKey, text)
                RemoteInput.addResultsToIntent(inputs, intent, bundle)
                action.actionIntent.send(this, 0, intent)
                // Also dismiss after reply?
                // cancelNotification(key)
                break
            }
        } catch(_: Exception) {}
    }
}
