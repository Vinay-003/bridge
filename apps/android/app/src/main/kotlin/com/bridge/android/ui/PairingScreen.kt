package com.bridge.android.ui

import android.content.Intent
import android.provider.Settings
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.edit
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import org.java_websocket.client.WebSocketClient
import org.java_websocket.handshake.ServerHandshake
import java.net.URI

@Composable
fun PairingScreen() {
    val ctx = LocalContext.current
    val prefs = remember { ctx.getSharedPreferences("bridge", 0) }
    var qr by remember { mutableStateOf(prefs.getString("last_qr","") ?: "") }
    var connected by remember { mutableStateOf(false) }
    var lastError by remember { mutableStateOf("") }
    var scannedHost by remember { mutableStateOf(prefs.getString("host","192.168.1.36") ?: "192.168.1.36") }
    var scannedPort by remember { mutableStateOf(prefs.getInt("port",8443)) }

    LaunchedEffect(Unit) {
        while(true) {
            kotlinx.coroutines.delay(2000)
            try {
                val c = WebSocketClientForCheck(URI("ws://$scannedHost:$scannedPort"))
                var ok=false
                c.onOpenCallback = { ok=true; connected=true }
                c.connectBlocking()
                kotlinx.coroutines.delay(1200)
                c.close()
                if(!ok) connected=false
            } catch(_: Exception) { connected=false }
        }
    }

    val launcher = rememberLauncherForActivityResult(ScanContract()) { result ->
        if(result.contents!=null) {
            qr = result.contents
            prefs.edit { putString("last_qr", qr) }
            try {
                val uri = android.net.Uri.parse(qr)
                val host = uri.getQueryParameter("host") ?: "192.168.1.36"
                val port = uri.getQueryParameter("port")?.toIntOrNull() ?: 8443
                val fp = uri.getQueryParameter("fp") ?: ""
                val id = uri.getQueryParameter("id") ?: ""
                scannedHost = host; scannedPort = port
                prefs.edit { putString("host", host); putInt("port", port); putString("fp", fp); putString("device_id", id) }
                val intent = Intent(ctx, com.bridge.android.service.BridgeService::class.java)
                intent.putExtra("host", host)
                intent.putExtra("port", port)
                ctx.startForegroundService(intent)
                connected = true
                Toast.makeText(ctx, "Scanned $host:$port fp $fp", Toast.LENGTH_SHORT).show()
            } catch(e: Exception) {
                lastError = e.message ?: "parse failed"
                Toast.makeText(ctx, "Invalid QR: $lastError", Toast.LENGTH_SHORT).show()
            }
        }
    }

    Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Pair with Linux", style = MaterialTheme.typography.headlineSmall)
        Text("1. Linux shows QR in Bridge tray (or /tmp/bridge-qr.png)\n2. Tap Scan QR\n3. Confirm 6-digit SAS matches desktop\n4. Done — auto-reconnect via mDNS/BLE", style = MaterialTheme.typography.bodyMedium)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = {
                val opts = ScanOptions()
                opts.setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                opts.setPrompt("Scan Bridge QR")
                opts.setBeepEnabled(true)
                opts.setOrientationLocked(false)
                launcher.launch(opts)
            }) { Text("Scan QR") }
            OutlinedButton(onClick = {
                // Stop / Disconnect
                try { ctx.stopService(Intent(ctx, com.bridge.android.service.BridgeService::class.java)) } catch(_:Exception){}
                connected = false
                Toast.makeText(ctx, "Bridge stopped", Toast.LENGTH_SHORT).show()
            }) { Text("Stop") }
        }
        if(qr.isNotEmpty()) {
            Card { Column(Modifier.padding(12.dp)) {
                Text(qr, style = MaterialTheme.typography.bodySmall)
                Text("Host $scannedHost:$scannedPort", style = MaterialTheme.typography.labelSmall)
                if(lastError.isNotEmpty()) Text(lastError, color=MaterialTheme.colorScheme.error)
            }}
        }
        Card(colors = CardDefaults.cardColors(containerColor = if(connected) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant)) {
            Column(Modifier.padding(16.dp)) {
                Text("Connected: $connected")
                Text("mDNS _bridge._tcp + BLE advertise active")
                Text("Daemon: ws://$scannedHost:$scannedPort", style = MaterialTheme.typography.labelSmall)
            }
        }
        OutlinedButton(onClick = {
            val intent = Intent(ctx, com.bridge.android.service.BridgeService::class.java)
            intent.putExtra("host", scannedHost)
            intent.putExtra("port", scannedPort)
            ctx.startForegroundService(intent)
            Toast.makeText(ctx, "Retrying $scannedHost:$scannedPort", Toast.LENGTH_SHORT).show()
        }) { Text("Enter 6-digit code instead / Retry connect") }
        Text("USB fallback: adb forward tcp:8443 tcp:8443", style = MaterialTheme.typography.labelSmall)
        Button(onClick = { ctx.startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)) }) { Text("Enable Notification Access") }
        Button(onClick = { ctx.startActivity(Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)) }) { Text("Disable Battery Optimization") }
        OutlinedButton(onClick = {
            try { ctx.stopService(Intent(ctx, com.bridge.android.service.BridgeService::class.java)) } catch(_:Exception){}
            Toast.makeText(ctx, "Bridge service stopped — notification will disappear", Toast.LENGTH_LONG).show()
        }) { Text("Stop Bridge Service") }
    }
}

class WebSocketClientForCheck(uri: URI): WebSocketClient(uri) {
    var onOpenCallback: (() -> Unit)? = null
    override fun onOpen(h: ServerHandshake?) { onOpenCallback?.invoke() }
    override fun onMessage(msg: String?) {}
    override fun onClose(code: Int, reason: String?, remote: Boolean) {}
    override fun onError(ex: Exception?) {}
}
