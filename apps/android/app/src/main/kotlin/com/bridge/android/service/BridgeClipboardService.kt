package com.bridge.android.service

import android.app.Service
import android.content.*
import android.os.IBinder
import android.content.ClipboardManager

class BridgeClipboardService : Service() {
    private var listener: ClipboardManager.OnPrimaryClipChangedListener? = null
    override fun onCreate() {
        super.onCreate()
        val cm = getSystemService(ClipboardManager::class.java)
        listener = ClipboardManager.OnPrimaryClipChangedListener {
            val text = cm.primaryClip?.getItemAt(0)?.text?.toString() ?: return@OnPrimaryClipChangedListener
            // send clipboard.sync via WS — stub
        }
        cm.addPrimaryClipChangedListener(listener!!)
    }
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onDestroy() {
        getSystemService(ClipboardManager::class.java)?.removePrimaryClipChangedListener(listener!!)
        super.onDestroy()
    }
}
