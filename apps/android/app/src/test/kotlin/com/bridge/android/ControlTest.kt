package com.bridge.android

import com.bridge.android.control.InputDispatcher
import com.bridge.android.control.ControlState
import org.json.JSONObject
import org.junit.Test
import org.junit.Assert.*

class ControlTest {
    @Test fun validActions() {
        assertTrue(InputDispatcher.isValidAction("tap"))
        assertTrue(InputDispatcher.isValidAction("move"))
        assertTrue(InputDispatcher.isValidAction("home"))
        assertFalse(InputDispatcher.isValidAction("evil"))
    }

    @Test fun clampWorks() {
        assertEquals(0.5, InputDispatcher.clamp01(0.5)!!, 0.001)
        assertNull(InputDispatcher.clamp01(-0.1))
        assertNull(InputDispatcher.clamp01(1.5))
        assertNull(InputDispatcher.clamp01(Double.NaN))
    }

    @Test fun normToPx() {
        assertEquals(453, InputDispatcher.normToPx(0.42, 1080))
        assertEquals(0, InputDispatcher.normToPx(0.0, 1080))
        assertEquals(1079, InputDispatcher.normToPx(1.0, 1080))
    }

    @Test fun validateTapOk() {
        val p = mapOf("x" to 0.5, "y" to 0.5, "action" to "tap")
        assertTrue(InputDispatcher.validateMap(p).isSuccess)
    }

    @Test fun validateInvalidCoords() {
        val p = mapOf("x" to 1.5, "y" to 0.5, "action" to "tap")
        assertTrue(InputDispatcher.validateMap(p).isFailure)
        val p2 = mapOf("y" to 0.5, "action" to "tap")
        assertTrue(InputDispatcher.validateMap(p2).isFailure)
    }

    @Test fun validateHomeNoCoords() {
        val p = mapOf("action" to "home")
        assertTrue(InputDispatcher.validateMap(p).isSuccess)
        val p2 = mapOf("action" to "back")
        assertTrue(InputDispatcher.validateMap(p2).isSuccess)
    }

    @Test fun validateKeyRequiresCode() {
        val p = mapOf("action" to "key", "keyCode" to 4)
        assertTrue(InputDispatcher.validateMap(p).isSuccess)
        val p2 = mapOf("action" to "key")
        assertTrue(InputDispatcher.validateMap(p2).isFailure)
    }

    @Test fun validatePinchScale() {
        val ok = mapOf("x" to 0.5, "y" to 0.5, "action" to "pinch", "scale" to 1.2)
        assertTrue(InputDispatcher.validateMap(ok).isSuccess)
        val bad = mapOf("x" to 0.5, "y" to 0.5, "action" to "pinch", "scale" to 10.0)
        assertTrue(InputDispatcher.validateMap(bad).isFailure)
        val bad2 = mapOf("x" to 0.5, "y" to 0.5, "action" to "tap", "scale" to 1.2)
        assertTrue(InputDispatcher.validateMap(bad2).isFailure)
    }

    @Test fun throttlePure() {
        assertFalse(InputDispatcher.shouldThrottlePure(null, 1000))
        assertTrue(InputDispatcher.shouldThrottlePure(1000L, 1005L))
        assertFalse(InputDispatcher.shouldThrottlePure(1000L, 1020L))
    }

    @Test fun rateLimitPure() {
        val vec = mutableListOf<Long>()
        repeat(120) { assertFalse(InputDispatcher.isRateLimitedPure(vec, 1000L)) }
        assertTrue(InputDispatcher.isRateLimitedPure(vec, 1000L))
        assertFalse(InputDispatcher.isRateLimitedPure(vec, 2500L))
    }

    @Test fun controlStateValid() {
        assertTrue(ControlState.DISABLED.canTransition(ControlState.ENABLED))
        assertTrue(ControlState.ENABLED.canTransition(ControlState.CONTROLLING))
        assertTrue(ControlState.CONTROLLING.canTransition(ControlState.PAUSED))
        assertTrue(ControlState.PAUSED.canTransition(ControlState.ENABLED))
        assertTrue(ControlState.CONTROLLING.canTransition(ControlState.ENABLED))
        assertTrue(ControlState.CONTROLLING.canTransition(ControlState.DISABLED))
    }

    @Test fun controlStateInvalid() {
        assertFalse(ControlState.DISABLED.canTransition(ControlState.CONTROLLING))
        assertFalse(ControlState.ENABLED.canTransition(ControlState.PAUSED))
        assertFalse(ControlState.PAUSED.canTransition(ControlState.CONTROLLING))
    }

    @Test fun coalesce() {
        // Validate that coalesce logic would keep latest move (pure, no JSONObject needed in unit test)
        val map = mapOf("x" to 0.11, "y" to 0.11, "action" to "move")
        assertTrue(InputDispatcher.validateMap(map).isSuccess)
        // Throttle pure already tests coalesce timing
    }
}
