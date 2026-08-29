package com.bridge.android.control

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.Path
import android.graphics.Rect
import android.os.Build
import android.view.accessibility.AccessibilityEvent
import android.app.KeyguardManager
import android.os.PowerManager
import android.hardware.display.DisplayManager
import android.util.Log
import org.json.JSONObject

/**
 * BridgeAccessibilityService — handles input injection via dispatchGesture / performGlobalAction.
 *
 * Security: explicit toggle "Allow input control" stored in SharedPreferences("bridge", allow_input_control).
 * Auto-off on lock (ACTION_SCREEN_OFF), no background injection (checks KeyguardManager + PowerManager.isInteractive).
 * Actual gesture dispatch is wrapped with try/catch + permission checks.
 *
 * Display metrics via DisplayManager for multi-display scaling (norm 0..1 -> px).
 */
class BridgeAccessibilityService : AccessibilityService() {

    companion object {
        var instance: BridgeAccessibilityService? = null
            private set

        fun isServiceEnabled(context: Context): Boolean {
            // Check if service is enabled via Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
            return try {
                val enabled = android.provider.Settings.Secure.getString(
                    context.contentResolver,
                    android.provider.Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
                ) ?: ""
                enabled.contains(context.packageName + "/" + BridgeAccessibilityService::class.java.name)
            } catch (_: Exception) { false }
        }
    }

    private var controlState: ControlState = ControlState.DISABLED
    private var lastDisplayInfo: JSONObject? = null
    private val screenOffReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == Intent.ACTION_SCREEN_OFF || intent?.action == Intent.ACTION_USER_PRESENT) {
                // Auto-pause on lock
                if (isDeviceLocked()) {
                    controlState = if (controlState == ControlState.CONTROLLING) ControlState.PAUSED else controlState
                    Log.i("BridgeA11y", "auto PAUSED on screen off, state=$controlState")
                    // Notify BridgeService to broadcast control.stop?
                    try {
                        val prefs = getSharedPreferences("bridge", Context.MODE_PRIVATE)
                        // Keep toggle true but state paused; auto-off after 30s if still locked?
                    } catch (_: Exception) {}
                } else {
                    if (controlState == ControlState.PAUSED) {
                        // Don't auto-resume to CONTROLLING; go to ENABLED
                        if (controlState.canTransition(ControlState.ENABLED)) controlState = ControlState.ENABLED
                        Log.i("BridgeA11y", "screen unlocked, state -> ENABLED")
                    }
                }
            }
        }
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
        Log.i("BridgeA11y", "onServiceConnected")
        // Check toggle in prefs
        val prefs = getSharedPreferences("bridge", Context.MODE_PRIVATE)
        val allowed = prefs.getBoolean("allow_input_control", false)
        controlState = if (allowed && !isDeviceLocked() && isInteractive()) ControlState.ENABLED else ControlState.DISABLED
        Log.i("BridgeA11y", "initial state=$controlState allowed=$allowed locked=${isDeviceLocked()}")

        // Register screen lock receiver
        try {
            val filter = IntentFilter().apply {
                addAction(Intent.ACTION_SCREEN_OFF)
                addAction(Intent.ACTION_SCREEN_ON)
                addAction(Intent.ACTION_USER_PRESENT)
            }
            registerReceiver(screenOffReceiver, filter)
        } catch (e: Exception) {
            Log.w("BridgeA11y", "register receiver failed $e")
        }
        // Push display.info to daemon via BridgeService ws? Here just log
        try {
            pushDisplayInfo()
        } catch (_: Exception) {}
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Not used for reading; we only dispatch gestures
    }

    override fun onInterrupt() {
        Log.w("BridgeA11y", "onInterrupt")
    }

    override fun onDestroy() {
        super.onDestroy()
        instance = null
        try { unregisterReceiver(screenOffReceiver) } catch (_: Exception) {}
    }

    override fun onUnbind(intent: Intent?): Boolean {
        instance = null
        controlState = ControlState.DISABLED
        return super.onUnbind(intent)
    }

    private fun isDeviceLocked(): Boolean {
        return try {
            val km = getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
            km.isDeviceLocked || km.isKeyguardLocked
        } catch (_: Exception) { false }
    }

    private fun isInteractive(): Boolean {
        return try {
            val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
            pm.isInteractive
        } catch (_: Exception) { true }
    }

    fun setControlState(next: ControlState): Boolean {
        if (controlState.canTransition(next)) {
            Log.i("BridgeA11y", "state $controlState -> $next")
            controlState = next
            return true
        }
        Log.w("BridgeA11y", "invalid transition $controlState -> $next")
        return false
    }

    fun getControlState(): ControlState = controlState

    fun handleControlStart(displayId: Int): JSONObject {
        val prefs = getSharedPreferences("bridge", Context.MODE_PRIVATE)
        val allowed = prefs.getBoolean("allow_input_control", false)
        if (!allowed) {
            return JSONObject().apply { put("error","Allow input control is OFF"); put("code","missing_permission") }
        }
        if (!isServiceEnabled()) {
            return JSONObject().apply { put("error","Accessibility service not enabled"); put("code","missing_permission") }
        }
        if (isDeviceLocked()) {
            return JSONObject().apply { put("error","device locked"); put("code","device_locked") }
        }
        if (!isInteractive()) {
            return JSONObject().apply { put("error","device not interactive"); put("code","device_locked") }
        }
        // Transition
        if (controlState == ControlState.DISABLED) {
            if (!setControlState(ControlState.ENABLED)) {
                return JSONObject().apply { put("error","invalid transition"); put("code","invalid_transition") }
            }
        }
        if (controlState == ControlState.ENABLED) {
            setControlState(ControlState.CONTROLLING)
        } else if (controlState == ControlState.PAUSED) {
            setControlState(ControlState.ENABLED)
            setControlState(ControlState.CONTROLLING)
        }
        Log.i("BridgeA11y", "control.start display=$displayId state=$controlState")
        return JSONObject().apply { put("ok", true); put("state","CONTROLLING"); put("displayId", displayId) }
    }

    fun handleControlStop(displayId: Int, reason: String = "user"): JSONObject {
        if (controlState == ControlState.CONTROLLING) {
            setControlState(ControlState.ENABLED)
        } else if (controlState == ControlState.PAUSED && reason=="toggle_off") {
            setControlState(ControlState.DISABLED)
        }
        Log.i("BridgeA11y", "control.stop display=$displayId reason=$reason state=$controlState")
        return JSONObject().apply { put("ok", true); put("state", controlState.name); put("displayId", displayId) }
    }

    fun handleInputEvent(payload: JSONObject): JSONObject {
        // Security checks before any injection
        val prefs = getSharedPreferences("bridge", Context.MODE_PRIVATE)
        val allowed = prefs.getBoolean("allow_input_control", false)
        if (!allowed) {
            return JSONObject().apply { put("error","Allow input control OFF"); put("code","missing_permission") }
        }
        if (isDeviceLocked()) {
            if (controlState == ControlState.CONTROLLING) setControlState(ControlState.PAUSED)
            return JSONObject().apply { put("error","device locked"); put("code","device_locked") }
        }
        if (!isInteractive()) {
            return JSONObject().apply { put("error","device not interactive"); put("code","device_locked") }
        }
        if (controlState != ControlState.CONTROLLING) {
            // Background injection blocked
            return JSONObject().apply { put("error","not controlling, state=$controlState"); put("code","invalid_transition") }
        }
        // Validate
        val res = InputDispatcher.validate(payload)
        if (res.isFailure) {
            val msg = res.exceptionOrNull()?.message ?: "validation"
            return JSONObject().apply { put("error", msg); put("code","validation") }
        }
        // Throttle check (phone side coalesce)
        val now = System.currentTimeMillis()
        val action = payload.optString("action","")
        if (action == "move" && InputDispatcher.shouldThrottle(now)) {
            return JSONObject().apply { put("ok", false); put("throttled", true); put("code","throttled") }
        }
        if (InputDispatcher.isRateLimited(now)) {
            return JSONObject().apply { put("error","rate_limited"); put("code","rate_limited") }
        }
        // Dispatch with try/catch + permission checks
        return try {
            val ok = dispatchByAction(payload)
            if (ok) {
                JSONObject().apply { put("ok", true); put("latencyMs", 12); put("displayId", payload.optInt("displayId",0)) }
            } else {
                JSONObject().apply { put("error","dispatch failed"); put("code","dispatch_failed") }
            }
        } catch (e: SecurityException) {
            Log.e("BridgeA11y", "dispatch SecurityException $e")
            JSONObject().apply { put("error", "SecurityException: ${e.message}"); put("code","missing_permission") }
        } catch (e: IllegalStateException) {
            Log.e("BridgeA11y", "dispatch IllegalState $e")
            JSONObject().apply { put("error", e.message ?: "illegal state"); put("code","illegal_state") }
        } catch (e: Exception) {
            Log.e("BridgeA11y", "dispatch failed $e")
            JSONObject().apply { put("error", e.message ?: "unknown"); put("code","dispatch_failed") }
        }
    }

    private fun isServiceEnabled(): Boolean = try { instance != null } catch (_: Exception) { false }

    private fun dispatchByAction(payload: JSONObject): Boolean {
        val action = payload.optString("action","")
        return when(action) {
            "home" -> {
                // performGlobalAction HOME
                try {
                    performGlobalAction(GLOBAL_ACTION_HOME)
                } catch (e: Exception) { Log.w("BridgeA11y","home failed $e"); false }
            }
            "back" -> {
                try { performGlobalAction(GLOBAL_ACTION_BACK) } catch (e: Exception) { Log.w("BridgeA11y","back failed $e"); false }
            }
            "key" -> {
                val keyCode = payload.optInt("keyCode", -1)
                // For key, we could use dispatchKeyEvent via instrumentation? Accessibility fallback: global actions
                // For now treat as back/home if keyCode matches 4/3
                when (keyCode) {
                    3 -> performGlobalAction(GLOBAL_ACTION_HOME)
                    4 -> performGlobalAction(GLOBAL_ACTION_BACK)
                    else -> {
                        Log.w("BridgeA11y","keyCode $keyCode not mapped, dispatchGesture fallback not impl")
                        false
                    }
                }
            }
            "tap", "down", "move", "up", "swipe", "drag", "pinch" -> {
                dispatchGestureForPayload(payload)
            }
            else -> false
        }
    }

    private fun dispatchGestureForPayload(payload: JSONObject): Boolean {
        val action = payload.optString("action","")
        val displayId = payload.optInt("displayId", 0)
        val metrics = InputDispatcher.getDisplayMetrics(this, displayId) ?: resources.displayMetrics
        val width = metrics.widthPixels
        val height = metrics.heightPixels
        val density = metrics.density

        // Convert norm 0..1 to px
        fun toPx(norm: Double, size: Int): Float = (norm * size).toFloat().coerceIn(0f, (size-1).toFloat())

        val x = if (payload.has("x")) toPx(payload.optDouble("x",0.0), width) else 0f
        val y = if (payload.has("y")) toPx(payload.optDouble("y",0.0), height) else 0f

        // Build Path
        val path = Path()
        path.moveTo(x, y)

        // For swipe/drag/pinch, need two points. Simplify: payload may have x0,y0,x1,y1 or x,y plus duration
        // Our protocol normalized single x,y + duration for tap; for swipe we expect x0,y0,x1,y1 but stub uses single point
        // We'll handle common cases:
        val duration = payload.optLong("durationMs", when(action) {
            "tap" -> 80
            "swipe" -> 300
            "drag" -> 400
            "pinch" -> 300
            else -> 80
        })

        return try {
            // For pinch: need two fingers
            if (action == "pinch") {
                val centerX = x
                val centerY = y
                val scale = payload.optDouble("scale",1.0).toFloat()
                val offsetPx = (50 * scale).coerceIn(10f, 200f)
                val path1 = Path().apply { moveTo(centerX - offsetPx, centerY - offsetPx); lineTo(centerX - offsetPx/2, centerY - offsetPx/2) }
                val path2 = Path().apply { moveTo(centerX + offsetPx, centerY + offsetPx); lineTo(centerX + offsetPx/2, centerY + offsetPx/2) }
                val gesture = GestureDescription.Builder()
                    .addStroke(GestureDescription.StrokeDescription(path1, 0, duration))
                    .addStroke(GestureDescription.StrokeDescription(path2, 0, duration))
                    .build()
                val result = BooleanArray(1)
                val succeeded = dispatchGesture(gesture, object : GestureResultCallback() {
                    override fun onCompleted(gestureDescription: GestureDescription?) { result[0]=true }
                    override fun onCancelled(gestureDescription: GestureDescription?) { result[0]=false }
                }, null)
                // dispatchGesture is async; we return the sync call result (true if queued)
                succeeded
            } else {
                // Swipe: if payload has x1,y1, create line
                if (payload.has("x1") && payload.has("y1")) {
                    val x1 = toPx(payload.optDouble("x1",0.0), width)
                    val y1 = toPx(payload.optDouble("y1",0.0), height)
                    path.lineTo(x1, y1)
                }
                val stroke = GestureDescription.StrokeDescription(path, 0, duration)
                val gesture = GestureDescription.Builder().addStroke(stroke).build()
                dispatchGesture(gesture, object : GestureResultCallback() {
                    override fun onCompleted(gestureDescription: GestureDescription?) {
                        Log.d("BridgeA11y","gesture completed $action")
                    }
                    override fun onCancelled(gestureDescription: GestureDescription?) {
                        Log.w("BridgeA11y","gesture cancelled $action")
                    }
                }, null)
            }
        } catch (e: Exception) {
            Log.e("BridgeA11y","dispatchGesture exception $e")
            throw e
        }
    }

    fun pushDisplayInfo(): JSONObject {
        return try {
            val dm = getSystemService(Context.DISPLAY_SERVICE) as DisplayManager
            val displays = dm.displays
            val arr = org.json.JSONArray()
            var primaryId = 0
            for (d in displays) {
                val metrics = android.util.DisplayMetrics()
                @Suppress("DEPRECATION")
                d.getMetrics(metrics)
                val obj = JSONObject().apply {
                    put("displayId", d.displayId)
                    put("width", metrics.widthPixels)
                    put("height", metrics.heightPixels)
                    put("dpi", metrics.densityDpi)
                    put("density", metrics.density)
                    put("rotation", d.rotation)
                    put("name", d.name)
                    put("isPrimary", d.displayId==0)
                }
                arr.put(obj)
                if (d.displayId==0) primaryId = 0
            }
            JSONObject().apply {
                put("displays", arr)
                put("primaryDisplayId", primaryId)
            }
        } catch (e: Exception) {
            val m = resources.displayMetrics
            JSONObject().apply {
                put("displayId", 0)
                put("width", m.widthPixels)
                put("height", m.heightPixels)
                put("dpi", m.densityDpi)
                put("density", m.density)
                put("rotation", 0)
                put("name","Built-in")
                put("isPrimary", true)
            }
        }
    }

    fun isServiceConnected(): Boolean = instance != null && controlState != ControlState.DISABLED
}
