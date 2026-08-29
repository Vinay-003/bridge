package com.bridge.android.ui

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import android.content.*
import android.os.BatteryManager

@Composable
fun StatusCards() {
    var status by remember { mutableStateOf("Collecting...") }
    LaunchedEffect(Unit) {
        // In real app collect via periodic WorkManager + send StatusPush via WS
        status = "Battery 87% charging • 31°C\nRAM 4.2/15.6 GB • Storage 120/512 GB\nSignal -67 dBm • WS 8443 • QUIC 8444"
    }
    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Device Status", style = MaterialTheme.typography.headlineSmall)
        Card { Text(status, Modifier.padding(16.dp)) }
        Text("Forwarded to Linux every 5s via status.push (encrypted TLS). Per-feature toggles in Paired Devices.", style = MaterialTheme.typography.bodySmall)
    }
}
