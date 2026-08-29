package com.bridge.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.platform.LocalContext
import android.content.Intent
import android.provider.Settings

@Composable
fun PairingScreen() {
    var qr by remember { mutableStateOf("") }
    var connected by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Pair with Linux", style = MaterialTheme.typography.headlineSmall)
        Text("1. Linux shows QR in Bridge tray\n2. Tap Scan QR\n3. Confirm 6-digit SAS matches\n4. Done — auto-reconnect via mDNS/BLE", style = MaterialTheme.typography.bodyMedium)
        Button(onClick = { /* launch MLKit BarcodeScanning + CameraX */ qr = "bridge://pair?v=1&id=demo" }) { Text("Scan QR") }
        if(qr.isNotEmpty()) Text("QR: $qr", style = MaterialTheme.typography.bodySmall)
        Card { Column(Modifier.padding(16.dp)) {
            Text("Connected: $connected")
            Text("mDNS _bridge._tcp + BLE advertise active")
        }}
        OutlinedButton(onClick = { }) { Text("Enter 6-digit code instead") }
        Text("USB fallback: adb forward tcp:8443 tcp:8443", style = MaterialTheme.typography.labelSmall)
        val ctx = LocalContext.current
        Button(onClick = { ctx.startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)) }) { Text("Enable Notification Access") }
        Button(onClick = { ctx.startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)) }) { Text("Disable Battery Optimization") }
    }
}
