package com.bridge.android

import com.bridge.android.ai.AiHandler
import org.junit.Assert.*
import org.junit.Test

class AiTest {
    @Test
    fun validateSummarizeOk() {
        val payload = mapOf("notifications" to listOf(mapOf("app" to "WhatsApp", "body" to "hello")), "maxLen" to 200)
        assertTrue(AiHandler.validateSummarize(payload).isSuccess)
    }
    @Test
    fun validateSummarizeEmpty() {
        val payload = mapOf("notifications" to emptyList<Any>(), "maxLen" to 200)
        assertTrue(AiHandler.validateSummarize(payload).isFailure)
    }
    @Test
    fun validateSummarizeTooMany() {
        val many = (0..20).map { mapOf("app" to "A", "body" to "hi") } // 21? 0..20 inclusive =21
        val payload = mapOf("notifications" to many)
        assertTrue(AiHandler.validateSummarize(payload).isFailure)
    }
    @Test
    fun validateTranscribeOk() {
        val b64 = java.util.Base64.getEncoder().encodeToString(ByteArray(100){0x42})
        val payload = mapOf("audio_b64" to b64, "format" to "opus", "lang" to "en")
        assertTrue(AiHandler.validateTranscribe(payload).isSuccess)
    }
    @Test
    fun validateTranscribeBadFormat() {
        val b64 = java.util.Base64.getEncoder().encodeToString(ByteArray(10){0x42})
        val payload = mapOf("audio_b64" to b64, "format" to "evil")
        assertTrue(AiHandler.validateTranscribe(payload).isFailure)
    }
    @Test
    fun aiStateTransitions() {
        assertTrue(AiHandler.AiState.IDLE.canTransition(AiHandler.AiState.QUEUED))
        assertTrue(AiHandler.AiState.QUEUED.canTransition(AiHandler.AiState.LOCAL))
        assertTrue(AiHandler.AiState.LOCAL.canTransition(AiHandler.AiState.DONE))
        assertTrue(AiHandler.AiState.DONE.canTransition(AiHandler.AiState.IDLE))
        assertFalse(AiHandler.AiState.IDLE.canTransition(AiHandler.AiState.DONE))
    }
    @Test
    fun localSummarize() {
        val notifs = listOf(mapOf("app" to "WhatsApp", "body" to "hello"), mapOf("app" to "Gmail", "body" to "test"))
        val s = AiHandler.localSummarize(notifs, 200)
        assertTrue(s.contains("2 notifications"))
        assertTrue(s.length <= 200)
    }
}
