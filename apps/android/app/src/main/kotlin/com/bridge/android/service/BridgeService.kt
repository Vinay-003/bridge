package com.bridge.android.service

import android.app.*
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import kotlinx.coroutines.*
import org.java_websocket.client.WebSocketClient
import org.java_websocket.handshake.ServerHandshake
import java.net.URI

class BridgeService : Service() {
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var wsClient: WebSocketClient? = null

    override fun onCreate() {
        super.onCreate()
        try {
            val notif = buildNotif("Bridge running — LAN discovery active")
            if (Build.VERSION.SDK_INT >= 34) {
                ServiceCompat.startForeground(this, 1, notif, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
            } else {
                startForeground(1, notif)
            }
        } catch (e: Exception) {
            // Fallback without foreground type (still show notification)
            try { startForeground(1, buildNotif("Bridge running")) } catch (_: Exception) {}
        }
        startDiscovery()
        connect()
    }

    private fun buildNotif(text: String): Notification {
        val chId = "bridge"
        val nm = getSystemService(NotificationManager::class.java)
        // create channel if missing
        try {
            nm.createNotificationChannel(NotificationChannel(chId, "Bridge", NotificationManager.IMPORTANCE_LOW))
        } catch (_: Exception) {}
        return NotificationCompat.Builder(this, chId)
            .setContentTitle("Bridge")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setOngoing(true)
            .build()
    }

    private fun startDiscovery() {
        // stub: NsdManager + BLE would go here
    }

    private fun connect() {
        val host = "192.168.1.36" // daemon LAN IP (update to your Linux IP)
        val uri = URI("ws://$host:8443")
        wsClient?.close()
        wsClient = object : WebSocketClient(uri) {
            override fun onOpen(h: ServerHandshake?) {}
            override fun onMessage(m: String?) { m?.let { handle(it) } }
            override fun onClose(code: Int, reason: String?, remote: Boolean) {
                scope.launch { delay(3000); connect() }
            }
            override fun onError(ex: Exception?) {}
        }.also {
            try { it.connect() } catch (_: Exception) {}
        }
    }

    private fun handle(json: String) {
        // TODO: route to file/clipboard/notify handlers
    }

    override fun onBind(intent: Intent?): IBinder? = null
    override fun onDestroy() { scope.cancel(); try{ wsClient?.close() }catch(_:Exception){}; super.onDestroy() }
}
