package com.bridge.android.ui

import android.content.Context
import android.content.Intent
import android.provider.Settings
import android.widget.Toast
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.bridge.android.control.BridgeAccessibilityService
import com.bridge.android.control.ControlState

@Composable
fun ControlScreen() {
    val ctx = LocalContext.current
    val prefs = remember { ctx.getSharedPreferences("bridge", Context.MODE_PRIVATE) }
    var allow by remember { mutableStateOf(prefs.getBoolean("allow_input_control", false)) }
    var svcEnabled by remember { mutableStateOf(BridgeAccessibilityService.isServiceEnabled(ctx)) }
    var state by remember { mutableStateOf(ControlState.DISABLED) }

    // Poll service state
    LaunchedEffect(Unit) {
        while (true) {
            kotlinx.coroutines.delay(2000)
            svcEnabled = BridgeAccessibilityService.isServiceEnabled(ctx)
            allow = prefs.getBoolean("allow_input_control", false)
            state = BridgeAccessibilityService.instance?.getControlState() ?: if (allow && svcEnabled) ControlState.ENABLED else ControlState.DISABLED
        }
    }

    Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text("Remote Control — Input Injection", style = MaterialTheme.typography.headlineSmall)
        Text("Allow desktop to control this phone via mouse/keyboard. Requires Accessibility Service.", style = MaterialTheme.typography.bodyMedium)

        Card(colors = CardDefaults.cardColors(containerColor = when(state) {
            ControlState.CONTROLLING -> MaterialTheme.colorScheme.primaryContainer
            ControlState.PAUSED -> MaterialTheme.colorScheme.tertiaryContainer
            ControlState.ENABLED -> MaterialTheme.colorScheme.secondaryContainer
            else -> MaterialTheme.colorScheme.surfaceVariant
        })) {
            Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("State: ${state.name}", style = MaterialTheme.typography.titleMedium)
                Text("Accessibility service: ${if(svcEnabled) "ENABLED" else "DISABLED"}")
                Text("Toggle: ${if(allow) "ON" else "OFF"}")
                Text("Auto-off on screen lock • No background injection • Throttle 60fps", style = MaterialTheme.typography.labelSmall)
            }
        }

        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
            Text("Allow input control", modifier = Modifier.weight(1f))
            Switch(checked = allow, onCheckedChange = { checked ->
                prefs.edit().putBoolean("allow_input_control", checked).apply()
                allow = checked
                if (checked) {
                    Toast.makeText(ctx, "Enable Bridge in Accessibility Settings", Toast.LENGTH_LONG).show()
                    try {
                        ctx.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).apply { addFlags(Intent.FLAG_ACTIVITY_NEW_TASK) })
                    } catch (_: Exception) {}
                } else {
                    // If turning off, also move to DISABLED
                    BridgeAccessibilityService.instance?.setControlState(ControlState.DISABLED)
                    Toast.makeText(ctx, "Input control OFF — injection blocked", Toast.LENGTH_SHORT).show()
                }
                svcEnabled = BridgeAccessibilityService.isServiceEnabled(ctx)
            })
        }

        if (!svcEnabled) {
            Button(onClick = {
                try { ctx.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).apply { addFlags(Intent.FLAG_ACTIVITY_NEW_TASK) }) } catch (e: Exception) {
                    Toast.makeText(ctx, "Open Settings > Accessibility > Bridge", Toast.LENGTH_LONG).show()
                }
            }) { Text("Open Accessibility Settings") }
            Text("Find 'Bridge' in Accessibility and enable it. You must toggle Allow ON first.", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }

        if (allow && svcEnabled) {
            Button(onClick = {
                val svc = BridgeAccessibilityService.instance
                if (svc != null) {
                    val res = svc.handleControlStart(0)
                    Toast.makeText(ctx, "Control start: $res", Toast.LENGTH_SHORT).show()
                } else {
                    Toast.makeText(ctx, "Service not connected yet", Toast.LENGTH_SHORT).show()
                }
            }) { Text("Test: Start Control (display 0)") }

            OutlinedButton(onClick = {
                val svc = BridgeAccessibilityService.instance
                val res = svc?.handleControlStop(0, "user") ?: "no service"
                Toast.makeText(ctx, "Control stop: $res", Toast.LENGTH_SHORT).show()
            }) { Text("Stop Control") }

            OutlinedButton(onClick = {
                // Simulate tap at center
                val svc = BridgeAccessibilityService.instance
                if (svc != null) {
                    val payload = org.json.JSONObject().apply {
                        put("x", 0.5); put("y", 0.5); put("action","tap"); put("displayId",0)
                    }
                    val res = svc.handleInputEvent(payload)
                    Toast.makeText(ctx, "Tap result: $res", Toast.LENGTH_SHORT).show()
                }
            }) { Text("Test Tap Center") }
        }

        Divider()
        Text("Permission matrix", style = MaterialTheme.typography.titleSmall)
        Text("BIND_ACCESSIBILITY_SERVICE — required for dispatchGesture / performGlobalAction (HOME/BACK). Grants only via System Settings.\nWRITE_SECURE_SETTINGS — optional, not required.", style = MaterialTheme.typography.bodySmall)

        Text("Threat model", style = MaterialTheme.typography.titleSmall)
        Text("• Toggle OFF by default • Auto-off when screen locked • No injection when app in background • Throttle 60fps, rate-limit 120/s • Audit log without coords", style = MaterialTheme.typography.bodySmall)

        Text("Display", style = MaterialTheme.typography.titleSmall)
        val info = try { BridgeAccessibilityService.instance?.pushDisplayInfo()?.toString(2) ?: "Service not connected" } catch(_:Exception){"err"}
        Card { Text(info, Modifier.padding(12.dp), style = MaterialTheme.typography.bodySmall) }

        Button(onClick = {
            try { ctx.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)) } catch(_:Exception){}
        }) { Text("Verify Accessibility Enabled") }
    }
}
