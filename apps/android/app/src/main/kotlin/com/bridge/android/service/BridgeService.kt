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
    private var currentHost = "192.168.1.36"
    private var currentPort = 8443

    override fun onCreate() {
        super.onCreate()
        // load saved host/port
        val prefs = getSharedPreferences("bridge", 0)
        currentHost = prefs.getString("host","192.168.1.36") ?: "192.168.1.36"
        currentPort = prefs.getInt("port",8443)
        try {
            val notif = buildNotif("Bridge running — LAN discovery active ($currentHost:$currentPort)")
            if (Build.VERSION.SDK_INT >= 34) {
                ServiceCompat.startForeground(this, 1, notif, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
            } else {
                startForeground(1, notif)
            }
        } catch (e: Exception) {
            try { startForeground(1, buildNotif("Bridge running")) } catch (_: Exception) {}
        }
        startDiscovery()
        connect()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        intent?.let {
            val h = it.getStringExtra("host")
            val p = it.getIntExtra("port", -1)
            if(h!=null) {
                currentHost = h
                if(p!=-1) currentPort = p
                getSharedPreferences("bridge",0).edit().putString("host", currentHost).putInt("port", currentPort).apply()
                // update notification
                val nm = getSystemService(NotificationManager::class.java)
                nm.notify(1, buildNotif("Bridge running — $currentHost:$currentPort"))
                // reconnect
                wsClient?.close()
                connect()
            }
        }
        return START_STICKY
    }

    private fun buildNotif(text: String): Notification {
        val chId = "bridge"
        val nm = getSystemService(NotificationManager::class.java)
        try { nm.createNotificationChannel(NotificationChannel(chId, "Bridge", NotificationManager.IMPORTANCE_LOW)) } catch (_: Exception) {}
        return NotificationCompat.Builder(this, chId)
            .setContentTitle("Bridge")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setOngoing(true)
            .build()
    }

    private fun startDiscovery() {
        // TODO: NsdManager mDNS + BLE
    }

    private fun connect() {
        val uri = URI("ws://$currentHost:$currentPort")
        wsClient?.close()
        wsClient = object : WebSocketClient(uri) {
            override fun onOpen(h: ServerHandshake?) {}
            override fun onMessage(m: String?) { m?.let { handle(it) } }
            override fun onClose(code: Int, reason: String?, remote: Boolean) {
                scope.launch { delay(3000); connect() }
            }
            override fun onError(ex: Exception?) {}
        }.also { try { it.connect() } catch (_: Exception) {} }
    }

    private fun handle(json: String) { }

    override fun onBind(intent: Intent?): IBinder? = null
    override fun onDestroy() { scope.cancel(); try{ wsClient?.close() }catch(_:Exception){}; super.onDestroy() }
}
