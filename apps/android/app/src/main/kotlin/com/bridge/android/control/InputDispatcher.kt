package com.bridge.android.control

import android.hardware.display.DisplayManager
import android.content.Context
import android.view.Display
import org.json.JSONObject

/**
 * Validate + coalesce + throttle 60fps, handle multi-display.
 * Pure logic separated for unit tests; Android DisplayManager part guarded.
 */
object InputDispatcher {
    const val THROTTLE_MS = 16L
    const val RATE_LIMIT_PER_SEC = 120
    private var lastInputTs: Long = 0L
    private val inputTimestamps = mutableListOf<Long>()

    fun isValidAction(action: String): Boolean = action in setOf(
        "tap", "down", "move", "up", "swipe", "pinch", "drag", "key", "home", "back"
    )

    fun clamp01(v: Double): Double? {
        if (!v.isFinite()) return null
        if (v < 0.0 || v > 1.0) return null
        return v
    }

    fun normToPx(norm: Double, sizePx: Int): Int {
        return (norm * sizePx).toInt().coerceIn(0, sizePx - 1)
    }

    fun shouldThrottle(now: Long): Boolean {
        val diff = now - lastInputTs
        if (diff < THROTTLE_MS) return true
        lastInputTs = now
        return false
    }

    // For testing: pure version
    fun shouldThrottlePure(last: Long?, now: Long, throttleMs: Long = THROTTLE_MS): Boolean {
        if (last == null) return false
        return now - last < throttleMs
    }

    fun isRateLimited(now: Long = System.currentTimeMillis()): Boolean {
        inputTimestamps.removeAll { now - it > 1000 }
        if (inputTimestamps.size >= RATE_LIMIT_PER_SEC) return true
        inputTimestamps.add(now)
        return false
    }

    fun isRateLimitedPure(vec: MutableList<Long>, now: Long, limit: Int = RATE_LIMIT_PER_SEC, windowMs: Long = 1000): Boolean {
        vec.removeAll { now - it > windowMs }
        if (vec.size >= limit) return true
        vec.add(now)
        return false
    }

    fun coalesceMoves(pending: JSONObject?, incoming: JSONObject): JSONObject {
        // For move actions, keep only latest
        if (pending != null && pending.optString("action") == "move" && incoming.optString("action") == "move") {
            return incoming
        }
        return incoming
    }

    fun validate(payload: JSONObject): Result<Unit> {
        // Convert JSONObject to Map for pure validation (avoids mock issues in tests via delegate)
        val map = mutableMapOf<String, Any?>()
        val keys = payload.keys()
        while (keys.hasNext()) {
            val k = keys.next()
            map[k] = payload.opt(k)
        }
        // Handle numbers as Double/Int
        return validateMap(map)
    }

    fun validateMap(map: Map<String, Any?>): Result<Unit> {
        val action = map["action"] as? String ?: ""
        if (!isValidAction(action)) return Result.failure(IllegalArgumentException("invalid action $action"))
        val needsCoords = action !in setOf("home", "back", "key")
        if (action == "key") {
            val kc = (map["keyCode"] as? Number)?.toLong() ?: return Result.failure(IllegalArgumentException("key requires keyCode"))
            if (kc < 0 || kc > 1000) return Result.failure(IllegalArgumentException("invalid keyCode $kc"))
        }
        if (needsCoords) {
            if (!map.containsKey("x") || !map.containsKey("y")) return Result.failure(IllegalArgumentException("missing x/y"))
            val x = (map["x"] as? Number)?.toDouble() ?: Double.NaN
            val y = (map["y"] as? Number)?.toDouble() ?: Double.NaN
            if (clamp01(x) == null) return Result.failure(IllegalArgumentException("invalid x $x"))
            if (clamp01(y) == null) return Result.failure(IllegalArgumentException("invalid y $y"))
        } else {
            if (map.containsKey("x")) {
                val x = (map["x"] as? Number)?.toDouble() ?: Double.NaN
                if (clamp01(x) == null) return Result.failure(IllegalArgumentException("invalid x $x"))
            }
            if (map.containsKey("y")) {
                val y = (map["y"] as? Number)?.toDouble() ?: Double.NaN
                if (clamp01(y) == null) return Result.failure(IllegalArgumentException("invalid y $y"))
            }
        }
        (map["pointerId"] as? Number)?.toLong()?.let {
            if (it < 0 || it > 9) return Result.failure(IllegalArgumentException("invalid pointerId"))
        }
        if (map.containsKey("pressure")) {
            val p = (map["pressure"] as? Number)?.toDouble() ?: -1.0
            if (!p.isFinite() || p < 0.0 || p > 1.0) return Result.failure(IllegalArgumentException("invalid pressure"))
        }
        if (map.containsKey("durationMs")) {
            val d = (map["durationMs"] as? Number)?.toLong() ?: -1
            if (d < 0 || d > 5000) return Result.failure(IllegalArgumentException("invalid duration"))
        }
        if (map.containsKey("scale")) {
            if (action != "pinch") return Result.failure(IllegalArgumentException("scale only for pinch"))
            val s = (map["scale"] as? Number)?.toDouble() ?: Double.NaN
            if (!s.isFinite() || s < 0.1 || s > 5.0) return Result.failure(IllegalArgumentException("invalid scale"))
        }
        if (map.containsKey("displayId")) {
            val did = (map["displayId"] as? Number)?.toLong() ?: -1
            if (did < 0) return Result.failure(IllegalArgumentException("invalid displayId"))
        }
        return Result.success(Unit)
    }

    fun getDisplayMetrics(context: Context, displayId: Int): android.util.DisplayMetrics? {
        return try {
            val dm = context.getSystemService(Context.DISPLAY_SERVICE) as DisplayManager
            val display = dm.getDisplay(displayId) ?: dm.displays.firstOrNull()
            display?.let {
                val metrics = android.util.DisplayMetrics()
                // Note: Display.getMetrics is deprecated but still works for compat
                @Suppress("DEPRECATION")
                it.getMetrics(metrics)
                metrics
            }
        } catch (_: Exception) {
            try {
                context.resources.displayMetrics
            } catch (_: Exception) { null }
        }
    }

    fun resetForTest() {
        lastInputTs = 0L
        inputTimestamps.clear()
    }
}

enum class ControlState { DISABLED, ENABLED, CONTROLLING, PAUSED;

    fun canTransition(to: ControlState): Boolean = when (this to to) {
        DISABLED to ENABLED -> true
        ENABLED to CONTROLLING -> true
        CONTROLLING to PAUSED -> true
        PAUSED to ENABLED -> true
        CONTROLLING to ENABLED -> true
        PAUSED to DISABLED -> true
        ENABLED to DISABLED -> true
        CONTROLLING to DISABLED -> true
        else -> false
    }
}
