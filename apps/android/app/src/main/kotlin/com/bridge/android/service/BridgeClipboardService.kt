package com.bridge.android.service

import android.app.Service
import android.content.*
import android.os.IBinder
import android.content.ClipboardManager

class BridgeClipboardService : Service() {
    private var listener: ClipboardManager.OnPrimaryClipChangedListener? = null
    private var lastSent = ""
    override fun onCreate() {
        super.onCreate()
        val cm = getSystemService(ClipboardManager::class.java)
        listener = ClipboardManager.OnPrimaryClipChangedListener {
            val text = cm.primaryClip?.getItemAt(0)?.text?.toString() ?: return@OnPrimaryClipChangedListener
            if (text == lastSent) return@OnPrimaryClipChangedListener
            if (text.length > 50000) return@OnPrimaryClipChangedListener
            lastSent = text
            // send via BridgeService WS — store in prefs for service to pick up, also broadcast
            val b64 = android.util.Base64.encodeToString(text.toByteArray(), android.util.Base64.NO_WRAP)
            val prefs = getSharedPreferences("bridge", 0)
            val host = prefs.getString("host","192.168.1.36") ?: "192.168.1.36"
            val port = prefs.getInt("port",8443)
            // send via simple WS client (ephemeral)
            Thread {
                try {
                    val uri = java.net.URI("ws://$host:$port")
                    val c = object: org.java_websocket.client.WebSocketClient(uri) {
                        override fun onOpen(h: org.java_websocket.handshake.ServerHandshake?) {
                            val payload = org.json.JSONObject().apply {
                                put("mime","text/plain")
                                put("data_b64", b64)
                                put("ts", System.currentTimeMillis())
                                put("source","android")
                            }
                            val msg = org.json.JSONObject().apply {
                                put("v",1); put("id", java.util.UUID.randomUUID().toString()); put("type","clipboard.sync"); put("ts", System.currentTimeMillis()); put("nonce","a"); put("payload", payload)
                            }
                            send(msg.toString())
                            close()
                        }
                        override fun onMessage(m: String?) {}
                        override fun onClose(c: Int, r: String?, re: Boolean) {}
                        override fun onError(e: Exception?) {}
                    }
                    c.connectBlocking()
                } catch(_: Exception) {}
            }.start()
        }
        cm.addPrimaryClipChangedListener(listener!!)
    }
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onDestroy() {
        getSystemService(ClipboardManager::class.java)?.removePrimaryClipChangedListener(listener!!)
        super.onDestroy()
    }
}
