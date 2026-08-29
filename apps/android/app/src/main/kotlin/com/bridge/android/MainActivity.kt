package com.bridge.android

import android.Manifest
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.bridge.android.ui.PairingScreen
import com.bridge.android.ui.StatusCards
import com.bridge.android.service.BridgeService

class MainActivity : ComponentActivity() {
    private val permLauncher = registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Request perms after compose is ready (avoid crash on deny)
        permLauncher.launch(arrayOf(
            Manifest.permission.CAMERA,
            Manifest.permission.RECORD_AUDIO,
            Manifest.permission.POST_NOTIFICATIONS
        ))
        // Start FGS safely — catch SecurityException on Android 14 if perms not yet granted
        try {
            val svc = Intent(this, BridgeService::class.java)
            if (android.os.Build.VERSION.SDK_INT >= 26) startForegroundService(svc) else startService(svc)
        } catch (e: Exception) { e.printStackTrace() }

        setContent {
            MaterialTheme {
                Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    BridgeRoot()
                }
            }
        }
    }
}

@Composable
fun BridgeRoot() {
    var tab by remember { mutableStateOf(0) }
    val tabs = listOf("Pair","Status","Files","Media")
    Column(Modifier.fillMaxSize()) {
        TabRow(selectedTabIndex = tab) {
            tabs.forEachIndexed { i, t -> Tab(selected = tab==i, onClick = {tab=i}, text = {Text(t)}) }
        }
        when(tab) {
            0 -> PairingScreen()
            1 -> StatusCards()
            2 -> FilesScreen()
            3 -> MediaScreen()
        }
    }
}

@Composable
fun FilesScreen() { Text("Files — drag from Linux will appear in /Download/Bridge. Tap to share to Linux.", Modifier.padding(16.dp)) }
@Composable
fun MediaScreen() { Text("Media — Camera/Mic/Screen bridge. Enable in BridgeService. v4l2loopback on Linux: /dev/video10", Modifier.padding(16.dp)) }
