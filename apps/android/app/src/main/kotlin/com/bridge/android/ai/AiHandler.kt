package com.bridge.android.ai

import android.content.Context
import android.util.Base64
import org.json.JSONObject
import java.io.File

object AiHandler {
    const val MAX_NOTIFS = 20
    const val MAX_AUDIO_DECODED = 5 * 1024 * 1024
    enum class AiState { IDLE, QUEUED, LOCAL, CLOUD, DONE, FAILED;
        fun canTransition(to: AiState): Boolean = when (this to to) {
            IDLE to QUEUED -> true
            QUEUED to LOCAL -> true
            QUEUED to CLOUD -> true
            QUEUED to FAILED -> true
            LOCAL to DONE -> true
            LOCAL to CLOUD -> true
            LOCAL to FAILED -> true
            CLOUD to DONE -> true
            CLOUD to FAILED -> true
            DONE to IDLE -> true
            FAILED to IDLE -> true
            else -> false
        }
    }

    fun localWhisperAvailable(ctx: Context? = null): Boolean {
        if (System.getenv("BRIDGE_LOCAL_AI") != null) return true
        return File("/usr/local/bin/whisper.cpp").exists() || File("/data/local/tmp/whisper.cpp").exists()
    }
    fun localLlamaAvailable(ctx: Context? = null): Boolean {
        if (System.getenv("BRIDGE_LOCAL_AI") != null) return true
        return File("/usr/local/bin/llama.cpp").exists() || File("/data/local/tmp/llama.cpp").exists()
    }

    fun validateSummarize(payload: Map<String, Any?>): Result<Unit> {
        val notifs = payload["notifications"] as? List<*> ?: return Result.failure(IllegalArgumentException("missing notifications"))
        if (notifs.isEmpty() || notifs.size > MAX_NOTIFS) return Result.failure(IllegalArgumentException("notifications len ${notifs.size} invalid"))
        var total = 0
        for (n in notifs) {
            val m = n as? Map<*,*> ?: return Result.failure(IllegalArgumentException("invalid notif"))
            val app = m["app"] as? String ?: return Result.failure(IllegalArgumentException("missing app"))
            val body = m["body"] as? String ?: ""
            if (app.isEmpty() || app.length > 64) return Result.failure(IllegalArgumentException("invalid app: $app"))
            if (body.length > 500) return Result.failure(IllegalArgumentException("body too long"))
            total += app.length + body.length + 50
        }
        if (total > 10*1024) return Result.failure(IllegalArgumentException("total chars >10k"))
        (payload["maxLen"] as? Number)?.let {
            val v = it.toLong()
            if (v==0L || v>1000) return Result.failure(IllegalArgumentException("invalid maxLen: $v"))
        }
        return Result.success(Unit)
    }

    fun validateTranscribe(payload: Map<String, Any?>): Result<Unit> {
        val b64 = payload["audio_b64"] as? String ?: return Result.failure(IllegalArgumentException("missing audio_b64"))
        if (b64.isEmpty() || b64.length > 7_000_000) return Result.failure(IllegalArgumentException("invalid audio_b64 len"))
        try {
            val decoded = try {
                Base64.decode(b64, Base64.DEFAULT)
            } catch (_: Exception) {
                try { java.util.Base64.getDecoder().decode(b64) } catch (_: Exception) { throw IllegalArgumentException("invalid base64") }
            }
            if (decoded.size > MAX_AUDIO_DECODED) return Result.failure(IllegalArgumentException("audio decoded >5MB"))
            if (decoded.isEmpty()) return Result.failure(IllegalArgumentException("audio empty"))
        } catch (e: IllegalArgumentException) {
            return Result.failure(e)
        } catch (_: Exception) {
            return Result.failure(IllegalArgumentException("invalid base64"))
        }
        val fmt = payload["format"] as? String ?: ""
        if (fmt !in listOf("opus","wav","mp3","m4a")) return Result.failure(IllegalArgumentException("invalid format: $fmt"))
        (payload["lang"] as? String)?.let {
            if (!Regex("^[a-z]{2}$").matches(it)) return Result.failure(IllegalArgumentException("invalid lang: $it"))
        }
        return Result.success(Unit)
    }

    fun validateResult(payload: Map<String, Any?>): Result<Unit> {
        val kind = payload["kind"] as? String ?: return Result.failure(IllegalArgumentException("missing kind"))
        if (kind !in listOf("summarize","transcribe")) return Result.failure(IllegalArgumentException("invalid kind: $kind"))
        val text = payload["text"] as? String ?: return Result.failure(IllegalArgumentException("missing text"))
        if (text.length > 5000) return Result.failure(IllegalArgumentException("text too long"))
        val model = payload["model"] as? String ?: return Result.failure(IllegalArgumentException("missing model"))
        if (model.isEmpty() || model.length>64) return Result.failure(IllegalArgumentException("invalid model"))
        return Result.success(Unit)
    }

    fun localSummarize(notifications: List<Map<String,String>>, maxLen: Int = 200): String {
        val perApp = mutableMapOf<String,Int>()
        for (n in notifications) {
            val app = n["app"] ?: "unknown"
            perApp[app] = (perApp[app]?:0)+1
        }
        val parts = perApp.entries.sortedBy { it.key }.joinToString(", ") { "${it.key}×${it.value}" }
        var summary = "${notifications.size} notifications: $parts"
        notifications.firstOrNull()?.get("body")?.let {
            summary += " — e.g., ${it.take(60)}"
        }
        return summary.take(maxLen)
    }

    fun handleSummarize(ctx: Context, payload: JSONObject): JSONObject {
        val map = mutableMapOf<String,Any?>()
        for (k in payload.keys()) map[k]=payload.opt(k)
        val v = validateSummarize(map)
        if (v.isFailure) throw v.exceptionOrNull()!!
        val notifs = payload.optJSONArray("notifications") ?: throw IllegalArgumentException("missing")
        val maxLen = payload.optInt("maxLen", 200)
        val requestId = payload.optString("requestId","unknown")
        val cloudConsent = payload.optBoolean("cloudConsent", false)
        val localAvail = localLlamaAvailable(ctx)
        if (localAvail) {
            val list = mutableListOf<Map<String,String>>()
            for (i in 0 until notifs.length()) {
                val o = notifs.getJSONObject(i)
                list.add(mapOf("app" to o.optString("app",""), "body" to o.optString("body","")))
            }
            val text = localSummarize(list, maxLen)
            return JSONObject().apply {
                put("requestId", requestId); put("kind","summarize"); put("text", text); put("model","llama.cpp-local"); put("tokens", text.split(" ").size); put("cached", false)
            }
        } else if (cloudConsent) {
            return JSONObject().apply {
                put("requestId", requestId); put("kind","summarize"); put("text","Cloud summary mock of ${notifs.length()} notifs"); put("model","gpt-4o-mini-cloud"); put("cached", false)
            }
        } else {
            throw IllegalStateException("cloud_consent_required")
        }
    }

    fun handleTranscribe(ctx: Context, payload: JSONObject): JSONObject {
        val map = mutableMapOf<String,Any?>()
        for (k in payload.keys()) map[k]=payload.opt(k)
        val v = validateTranscribe(map)
        if (v.isFailure) throw v.exceptionOrNull()!!
        val b64 = payload.getString("audio_b64")
        val fmt = payload.getString("format")
        val lang = payload.optString("lang","en")
        val requestId = payload.optString("requestId","unknown")
        val cloudConsent = payload.optBoolean("cloudConsent", false)
        val localAvail = localWhisperAvailable(ctx)
        if (localAvail) {
            val decodedSize = try { Base64.decode(b64, Base64.DEFAULT).size } catch (_: Exception) { try { java.util.Base64.getDecoder().decode(b64).size } catch (_: Exception) { b64.length } }
            val text = "Transcribed $decodedSize bytes format $fmt lang $lang mock whisper.cpp"
            return JSONObject().apply { put("requestId",requestId); put("kind","transcribe"); put("text",text); put("model","whisper.cpp-local") }
        } else if (cloudConsent) {
            return JSONObject().apply { put("requestId",requestId); put("kind","transcribe"); put("text","Cloud transcribed mock whisper-1"); put("model","whisper-1-cloud") }
        } else {
            throw IllegalStateException("cloud_consent_required")
        }
    }
}
