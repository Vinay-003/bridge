package com.bridge.android

import android.Manifest
import android.content.Intent
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.bridge.android.ui.ControlScreen
import com.bridge.android.ui.PairingScreen
import com.bridge.android.ui.StatusCards
import com.bridge.android.service.BridgeService

class MainActivity : ComponentActivity() {
    private var lastError: String? = null
    private val permLauncher = registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { }

    override fun onCreate(savedInstanceState: Bundle?) {
        Thread.setDefaultUncaughtExceptionHandler { _, e ->
            lastError = e.message ?: e.toString()
            try { Toast.makeText(this, "Crash: $lastError", Toast.LENGTH_LONG).show() } catch(_:Exception){}
            e.printStackTrace()
        }
        try {
            super.onCreate(savedInstanceState)
        } catch(e: Exception) {
            lastError = e.message
            Toast.makeText(this, "onCreate super failed: $e", Toast.LENGTH_LONG).show()
            return
        }

        // Request perms safely
        try {
            permLauncher.launch(arrayOf(
                Manifest.permission.CAMERA,
                Manifest.permission.RECORD_AUDIO,
                Manifest.permission.POST_NOTIFICATIONS
            ))
        } catch(e: Exception) { lastError = "perm: $e" }

        // Start FGS safely — catch everything
        try {
            val svc = Intent(this, BridgeService::class.java)
            if (android.os.Build.VERSION.SDK_INT >= 26) startForegroundService(svc) else startService(svc)
        } catch (e: Exception) {
            lastError = "FGS: $e"
            Toast.makeText(this, "FGS start failed (will retry after perms): $e", Toast.LENGTH_LONG).show()
        }

        setContent {
            MaterialTheme {
                Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    if(lastError!=null) {
                        Column(Modifier.padding(16.dp)) {
                            Text("Bridge start error", color = MaterialTheme.colorScheme.error)
                            Text(lastError!!)
                            Button(onClick = {
                                try {
                                    val svc = Intent(this@MainActivity, BridgeService::class.java)
                                    if (android.os.Build.VERSION.SDK_INT >= 26) startForegroundService(svc) else startService(svc)
                                    lastError = null
                                } catch(e: Exception){ lastError = e.toString() }
                            }) { Text("Retry start") }
                        }
                    } else {
                        BridgeRoot()
                    }
                }
            }
        }
    }
}

@Composable
fun BridgeRoot() {
    var tab by remember { mutableStateOf(0) }
    val tabs = listOf("Pair","Status","Files","Media","Control")
    Column(Modifier.fillMaxSize()) {
        TabRow(selectedTabIndex = tab) {
            tabs.forEachIndexed { i, t -> Tab(selected = tab==i, onClick = {tab=i}, text = {Text(t)}) }
        }
        when(tab) {
            0 -> PairingScreen()
            1 -> StatusCards()
            2 -> FilesScreen()
            3 -> MediaScreen()
            4 -> ControlScreen()
        }
    }
}

@Composable
fun FilesScreen() { Text("Files — drag from Linux will appear in /Download/Bridge. Tap to share to Linux.", Modifier.padding(16.dp)) }
@Composable
fun MediaScreen() { Text("Media — Camera/Mic/Screen bridge. Enable in BridgeService. v4l2loopback on Linux: /dev/video10", Modifier.padding(16.dp)) }
