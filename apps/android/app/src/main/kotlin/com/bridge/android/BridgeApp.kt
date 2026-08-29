package com.bridge.android

import android.app.Application
import android.content.Intent
import com.bridge.android.service.BridgeService

class BridgeApp : Application() {
    override fun onCreate() {
        super.onCreate()
        startService(Intent(this, BridgeService::class.java))
    }
}
