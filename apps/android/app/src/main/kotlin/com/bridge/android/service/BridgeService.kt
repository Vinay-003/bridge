package com.bridge.android.service

import android.app.*
import android.content.Intent
import android.os.IBinder
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*
import okhttp3.*
import org.java_websocket.client.WebSocketClient
import org.java_websocket.handshake.ServerHandshake
import java.net.URI

class BridgeService : Service() {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var wsClient: WebSocketClient? = null

    override fun onCreate() {
        super.onCreate()
        startForeground(1, buildNotif("Bridge running — LAN discovery active"))
        startDiscovery()
        connect()
    }

    private fun buildNotif(text: String): Notification {
        val chId = "bridge"
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(NotificationChannel(chId, "Bridge", NotificationManager.IMPORTANCE_LOW))
        return NotificationCompat.Builder(this, chId)
            .setContentTitle("Bridge")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setOngoing(true)
            .build()
    }

    private fun startDiscovery() {
        // mDNS via NsdManager + BLE advertise (omitted for brevity — stub)
        // Would register _bridge._tcp via NsdManager
    }

    private fun connect() {
        // Try LAN hosts via mDNS discovery; fallback to manual IP stored in DataStore
        val host = "192.168.1.50" // TODO: discovered via NsdManager
        val uri = URI("ws://$host:8443")
        wsClient = object : WebSocketClient(uri) {
            override fun onOpen(h: ServerHandshake?) { }
            override fun onMessage(m: String?) {
                // route BridgeMessage types: file.chunk, clipboard.sync, notify.action, webrtc.* etc.
                m?.let { handle(it) }
            }
            override fun onClose(code: Int, reason: String?, remote: Boolean) {
                scope.launch { delay(3000); connect() }
            }
            override fun onError(ex: Exception?) { }
        }.also { it.connect() }
    }

    private fun handle(json: String) {
        // Minimal router: echo status, file, clipboard
        // Full impl mirrors bridge-core protocol with DataStore persistence
    }

    override fun onBind(intent: Intent?): IBinder? = null
    override fun onDestroy() { scope.cancel(); wsClient?.close(); super.onDestroy() }
}
