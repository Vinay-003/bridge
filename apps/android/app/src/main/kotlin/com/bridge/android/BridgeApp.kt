package com.bridge.android

import android.app.Application

class BridgeApp : Application() {
    override fun onCreate() {
        super.onCreate()
        // Do NOT auto-start foreground service here (Android 14 blocks background FGS)
        // Service started from MainActivity after permissions granted
    }
}
