package com.bridge.android.storage

import android.content.Context
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import androidx.documentfile.provider.DocumentFile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.security.MessageDigest

object StorageHandler {
    const val CHUNK_SIZE = 1024 * 1024
    const val MAX_SIZE_BYTES = 50L * 1024 * 1024 * 1024

    enum class StorageState { IDLE, SCANNING, SYNCING, CONFLICT, DONE;
        fun canTransition(to: StorageState): Boolean = when (this to to) {
            IDLE to SCANNING -> true
            IDLE to DONE -> true
            SCANNING to SYNCING -> true
            SCANNING to DONE -> true
            SYNCING to CONFLICT -> true
            SYNCING to DONE -> true
            SYNCING to IDLE -> true
            CONFLICT to SYNCING -> true
            CONFLICT to IDLE -> true
            DONE to IDLE -> true
            else -> false
        }
    }

    fun sanitizePhonePath(path: String): String {
        if (path.isEmpty()) throw IllegalArgumentException("path empty")
        if (path.contains('\u0000')) throw IllegalArgumentException("path contains NUL")
        if (path.length > 4096) throw IllegalArgumentException("path too long")
        if (path == "/" || path == "") return ""
        val parts = mutableListOf<String>()
        for (seg in path.split("/")) {
            if (seg.isEmpty() || seg == ".") continue
            if (seg == "..") throw IllegalArgumentException("path traversal: $path")
            if (seg.length > 255) throw IllegalArgumentException("segment too long")
            parts.add(seg)
        }
        return parts.joinToString("/")
    }

    fun validateLs(payload: Map<String, Any?>): Result<Unit> {
        val path = payload["path"] as? String ?: return Result.failure(IllegalArgumentException("missing path"))
        return try {
            sanitizePhonePath(path)
            Result.success(Unit)
        } catch (e: Exception) {
            if (path == "/") Result.success(Unit) else Result.failure(e)
        }
    }

    fun validateSync(payload: Map<String, Any?>): Result<Unit> {
        val path = payload["path"] as? String ?: return Result.failure(IllegalArgumentException("missing path"))
        if (path == "/") return Result.failure(IllegalArgumentException("cannot sync root"))
        try { sanitizePhonePath(path) } catch (e: Exception) { return Result.failure(e) }
        val sha = payload["sha256"] as? String ?: return Result.failure(IllegalArgumentException("missing sha256"))
        if (sha.length != 64 || !sha.all { it.isDigit() || it.lowercaseChar() in 'a'..'f' }) {
            return Result.failure(IllegalArgumentException("invalid sha256: $sha"))
        }
        val offset = (payload["offset"] as? Number)?.toLong() ?: return Result.failure(IllegalArgumentException("missing offset"))
        val size = (payload["size"] as? Number)?.toLong() ?: return Result.failure(IllegalArgumentException("missing size"))
        if (size == 0L) return Result.failure(IllegalArgumentException("size 0"))
        if (size > MAX_SIZE_BYTES) return Result.failure(IllegalArgumentException("size > 50GiB"))
        if (offset >= size) return Result.failure(IllegalArgumentException("offset $offset >= size $size"))
        val total = (payload["total"] as? Number)?.toLong() ?: return Result.failure(IllegalArgumentException("missing total"))
        val index = (payload["index"] as? Number)?.toLong() ?: return Result.failure(IllegalArgumentException("missing index"))
        if (total == 0L) return Result.failure(IllegalArgumentException("total 0"))
        if (index >= total) return Result.failure(IllegalArgumentException("index $index >= total $total"))
        if (offset != index * CHUNK_SIZE) {
            return Result.failure(IllegalArgumentException("offset $offset != index $index * chunk $CHUNK_SIZE"))
        }
        return Result.success(Unit)
    }

    fun validateMkdir(payload: Map<String, Any?>): Result<Unit> {
        val path = payload["path"] as? String ?: return Result.failure(IllegalArgumentException("missing path"))
        if (path == "/") return Result.failure(IllegalArgumentException("cannot mkdir root"))
        return try { sanitizePhonePath(path); Result.success(Unit) } catch (e: Exception) { Result.failure(e) }
    }

    fun validateRm(payload: Map<String, Any?>): Result<Unit> {
        val path = payload["path"] as? String ?: return Result.failure(IllegalArgumentException("missing path"))
        if (path == "/") return Result.failure(IllegalArgumentException("cannot rm root"))
        return try { sanitizePhonePath(path); Result.success(Unit) } catch (e: Exception) { Result.failure(e) }
    }

    fun dominatesVector(a: Map<String, Long>, b: Map<String, Long>): Boolean {
        var allGe = true
        var strictlyGreater = false
        for ((k, bv) in b) {
            val av = a[k] ?: 0L
            if (av < bv) { allGe = false; break }
            if (av > bv) strictlyGreater = true
        }
        if (!allGe) return false
        for ((k, av) in a) {
            if (!b.containsKey(k) && av > 0) { strictlyGreater = true; break }
        }
        return strictlyGreater
    }

    fun isConcurrent(a: Map<String, Long>, b: Map<String, Long>): Boolean {
        if (vectorsEqual(a, b)) return false
        return !dominatesVector(a, b) && !dominatesVector(b, a)
    }

    private fun vectorsEqual(a: Map<String, Long>, b: Map<String, Long>): Boolean {
        val keys = (a.keys + b.keys)
        for (k in keys) if ((a[k] ?: 0L) != (b[k] ?: 0L)) return false
        return true
    }

    fun mergeVector(a: Map<String, Long>, b: Map<String, Long>): Map<String, Long> {
        val out = a.toMutableMap()
        for ((k, bv) in b) out[k] = maxOf(out[k] ?: 0L, bv)
        return out
    }

    fun formatTrashInfo(originalPath: String, deletionDate: String): String {
        return "[Trash Info]\nPath=$originalPath\nDeletionDate=$deletionDate\n"
    }

    fun parseTrashInfo(content: String): Map<String, String> {
        val out = mutableMapOf<String, String>()
        for (line in content.lines()) {
            val t = line.trim()
            if (t.isEmpty() || t.startsWith("[Trash") || t.startsWith("#")) continue
            val eq = t.indexOf('=')
            if (eq > 0) out[t.substring(0, eq).trim()] = t.substring(eq + 1).trim()
        }
        return out
    }

    fun resolveDocumentFile(ctx: Context, sanitizedRel: String, treeUri: Uri?): DocumentFile? {
        if (treeUri != null) {
            try {
                val tree = DocumentFile.fromTreeUri(ctx, treeUri) ?: return null
                if (sanitizedRel.isEmpty()) return tree
                var cur: DocumentFile = tree
                for (seg in sanitizedRel.split("/")) {
                    if (seg.isEmpty()) continue
                    val next = cur.findFile(seg) ?: return null
                    cur = next
                }
                return cur
            } catch (_: Exception) {}
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && Environment.isExternalStorageManager()) {
            val root = Environment.getExternalStorageDirectory()
            val f = if (sanitizedRel.isEmpty()) root else File(root, sanitizedRel)
            return DocumentFile.fromFile(f)
        }
        return null
    }

    fun lsViaSaf(ctx: Context, sanitizedRel: String, showHidden: Boolean, recursive: Boolean): List<Map<String, Any>> {
        val entries = mutableListOf<Map<String, Any>>()
        val prefs = ctx.getSharedPreferences("bridge", 0)
        val treeUriStr = prefs.getString("saf_tree_uri", null)
        val treeUri: Uri? = treeUriStr?.let { try { Uri.parse(it) } catch (_: Exception) { null } }
        val doc = resolveDocumentFile(ctx, sanitizedRel, treeUri)
        if (doc != null && doc.exists()) {
            val isFileBacked = Build.VERSION.SDK_INT >= 30 && Environment.isExternalStorageManager()
            if (isFileBacked) {
                val file = File(Environment.getExternalStorageDirectory(), sanitizedRel)
                val list = file.listFiles() ?: emptyArray()
                for (f in list) {
                    val fname: String = f.name
                    if (!showHidden && fname.startsWith(".")) continue
                    entries.add(mapOf(
                        "name" to fname,
                        "path" to (if (sanitizedRel.isEmpty()) "/$fname" else "/$sanitizedRel/$fname"),
                        "isDir" to f.isDirectory,
                        "size" to f.length(),
                        "mtimeMs" to f.lastModified()
                    ))
                }
                entries.sortWith(compareBy<Map<String, Any>>({ !(it["isDir"] as Boolean) }, { it["name"] as String }))
                return entries
            }
            val children: Array<DocumentFile> = try { doc.listFiles() } catch (_: Exception) { emptyArray() }
            for (c in children) {
                val name: String = c.name ?: continue
                if (!showHidden && name.startsWith(".")) continue
                val path = if (sanitizedRel.isEmpty()) "/$name" else "/$sanitizedRel/$name"
                entries.add(mapOf(
                    "name" to name,
                    "path" to path,
                    "isDir" to c.isDirectory,
                    "size" to c.length(),
                    "mtimeMs" to c.lastModified()
                ))
                if (recursive && c.isDirectory) {
                    val subRel = if (sanitizedRel.isEmpty()) name else "$sanitizedRel/$name"
                    entries.addAll(lsViaSaf(ctx, subRel, showHidden, false))
                }
            }
            entries.sortWith(compareBy<Map<String, Any>>({ !(it["isDir"] as Boolean) }, { it["name"] as String }))
            if (entries.isNotEmpty()) return entries
        }
        // Fallback MediaStore for DCIM
        if (sanitizedRel.startsWith("DCIM") || sanitizedRel.startsWith("Pictures") || sanitizedRel.startsWith("Movies") || sanitizedRel.isEmpty()) {
            try {
                val collection = if (Build.VERSION.SDK_INT >= 29) MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL) else MediaStore.Images.Media.EXTERNAL_CONTENT_URI
                val projection = arrayOf(MediaStore.MediaColumns.DISPLAY_NAME, MediaStore.MediaColumns.SIZE, MediaStore.MediaColumns.DATE_MODIFIED, MediaStore.MediaColumns.MIME_TYPE)
                ctx.contentResolver.query(collection, projection, null, null, null)?.use { cursor ->
                    val idxName = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.DISPLAY_NAME)
                    val idxSize = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.SIZE)
                    val idxDate = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.DATE_MODIFIED)
                    val idxMime = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.MIME_TYPE)
                    var count = 0
                    while (cursor.moveToNext() && count < 5000) {
                        val name: String = cursor.getString(idxName) ?: continue
                        if (!showHidden && name.startsWith(".")) continue
                        val size = cursor.getLong(idxSize)
                        val mtime = cursor.getLong(idxDate) * 1000
                        val mime: String = cursor.getString(idxMime) ?: ""
                        entries.add(mapOf(
                            "name" to name,
                            "path" to "/DCIM/$name",
                            "isDir" to false,
                            "size" to size,
                            "mtimeMs" to mtime,
                            "mime" to mime
                        ))
                        count++
                    }
                }
            } catch (_: Exception) {}
        }
        entries.sortWith(compareBy<Map<String, Any>>({ !(it["isDir"] as Boolean) }, { it["name"] as String }))
        return entries
    }

    fun handleLs(ctx: Context, payload: JSONObject): JSONObject {
        val path = payload.optString("path", "/")
        val showHidden = payload.optBoolean("showHidden", false)
        val recursive = payload.optBoolean("recursive", false)
        val rel = sanitizePhonePath(path)
        val entries = lsViaSaf(ctx, rel, showHidden, recursive)
        val arr = JSONArray()
        for (e in entries) {
            val o = JSONObject()
            o.put("name", e["name"])
            o.put("path", e["path"])
            o.put("isDir", e["isDir"])
            o.put("size", e["size"])
            o.put("mtimeMs", e["mtimeMs"])
            (e["mime"] as? String)?.let { o.put("mime", it) }
            arr.put(o)
        }
        return JSONObject().apply {
            put("path", path)
            put("entries", arr)
            put("truncated", entries.size >= 5000)
        }
    }

    fun handleStat(ctx: Context, payload: JSONObject): JSONObject {
        val path = payload.optString("path", "/")
        val rel = sanitizePhonePath(path)
        val prefs = ctx.getSharedPreferences("bridge", 0)
        val treeUriStr = prefs.getString("saf_tree_uri", null)
        val treeUri: Uri? = treeUriStr?.let { try { Uri.parse(it) } catch (_: Exception) { null } }
        val doc = resolveDocumentFile(ctx, rel, treeUri)
        if (doc == null || !doc.exists()) {
            if (Build.VERSION.SDK_INT >= 30 && Environment.isExternalStorageManager()) {
                val f = if (rel.isEmpty()) Environment.getExternalStorageDirectory() else File(Environment.getExternalStorageDirectory(), rel)
                if (f.exists()) {
                    return JSONObject().apply {
                        put("path", path)
                        put("isDir", f.isDirectory)
                        put("size", f.length())
                        put("mtimeMs", f.lastModified())
                        put("exists", true)
                    }
                }
            }
            return JSONObject().apply { put("path", path); put("exists", false) }
        }
        return JSONObject().apply {
            put("path", path)
            put("isDir", doc.isDirectory)
            put("size", doc.length())
            put("mtimeMs", doc.lastModified())
            put("exists", true)
            put("mime", doc.type ?: "")
        }
    }

    fun handleMkdir(ctx: Context, payload: JSONObject): JSONObject {
        val path = payload.optString("path", "")
        if (path == "/" || path.isEmpty()) throw IllegalArgumentException("cannot mkdir root")
        val rel = sanitizePhonePath(path)
        val prefs = ctx.getSharedPreferences("bridge", 0)
        val treeUriStr = prefs.getString("saf_tree_uri", null)
        val treeUri: Uri? = treeUriStr?.let { try { Uri.parse(it) } catch (_: Exception) { null } }
        if (treeUri != null) {
            val tree = DocumentFile.fromTreeUri(ctx, treeUri) ?: throw IllegalStateException("SAF tree not available")
            var cur = tree
            for (seg in rel.split("/")) {
                if (seg.isEmpty()) continue
                var next = cur.findFile(seg)
                if (next == null) next = cur.createDirectory(seg) ?: throw IllegalStateException("mkdir failed for $seg")
                cur = next
            }
            return JSONObject().apply { put("ok", true); put("path", path) }
        }
        if (Build.VERSION.SDK_INT >= 30 && Environment.isExternalStorageManager()) {
            val f = File(Environment.getExternalStorageDirectory(), rel)
            if (!f.exists() && !f.mkdirs()) throw IllegalStateException("mkdirs failed")
            return JSONObject().apply { put("ok", true); put("path", path) }
        }
        throw IllegalStateException("missing_permission: SAF or MANAGE_EXTERNAL_STORAGE required")
    }

    fun handleRm(ctx: Context, payload: JSONObject): JSONObject {
        val path = payload.optString("path", "")
        val toTrash = payload.optBoolean("toTrash", true)
        if (path == "/" || path.isEmpty()) throw IllegalArgumentException("cannot rm root")
        val rel = sanitizePhonePath(path)
        if (toTrash && Build.VERSION.SDK_INT >= 30) {
            try {
                val resolver = ctx.contentResolver
                val collection = MediaStore.Files.getContentUri("external")
                val name = rel.substringAfterLast("/")
                val cursor = resolver.query(collection, arrayOf(MediaStore.MediaColumns._ID), "${MediaStore.MediaColumns.DISPLAY_NAME}=?", arrayOf(name), null)
                cursor?.use {
                    if (it.moveToFirst()) {
                        val id = it.getLong(0)
                        val uri = MediaStore.Files.getContentUri("external", id)
                        try {
                            val values = android.content.ContentValues().apply { put(MediaStore.MediaColumns.IS_TRASHED, 1) }
                            resolver.update(uri, values, null, null)
                        } catch (_: Exception) {}
                        return JSONObject().apply { put("ok", true); put("path", path); put("trashed", true) }
                    }
                }
            } catch (_: Exception) {}
        }
        val prefs = ctx.getSharedPreferences("bridge", 0)
        val treeUriStr = prefs.getString("saf_tree_uri", null)
        val treeUri: Uri? = treeUriStr?.let { try { Uri.parse(it) } catch (_: Exception) { null } }
        val doc = resolveDocumentFile(ctx, rel, treeUri)
        if (doc != null && doc.exists()) {
            val deleted = try { doc.delete() } catch (_: Exception) { false }
            if (!deleted) throw IllegalStateException("delete failed")
            return JSONObject().apply { put("ok", true); put("path", path); put("trashed", toTrash) }
        }
        if (Build.VERSION.SDK_INT >= 30 && Environment.isExternalStorageManager()) {
            val f = File(Environment.getExternalStorageDirectory(), rel)
            if (f.exists()) {
                val ok = if (f.isDirectory) f.deleteRecursively() else f.delete()
                if (!ok) throw IllegalStateException("file delete failed")
                return JSONObject().apply { put("ok", true); put("path", path); put("trashed", false) }
            }
        }
        throw IllegalStateException("not_found: $path")
    }

    fun handleSyncChunk(ctx: Context, payload: JSONObject): JSONObject {
        val map = mutableMapOf<String, Any?>()
        for (k in payload.keys()) map[k] = payload.opt(k)
        val res = validateSync(map)
        if (res.isFailure) throw res.exceptionOrNull()!!
        val path = payload.getString("path")
        val rel = sanitizePhonePath(path)
        val offset = payload.getLong("offset")
        val shaClaim = payload.getString("sha256")
        val b64 = payload.optString("data_b64", "")
        val bytes: ByteArray = try {
            android.util.Base64.decode(b64, android.util.Base64.DEFAULT)
        } catch (_: Exception) {
            try { java.util.Base64.getDecoder().decode(b64) } catch (_: Exception) { ByteArray(0) }
        }
        val md = MessageDigest.getInstance("SHA-256")
        val got = md.digest(bytes).joinToString("") { "%02x".format(it) }
        if (got.lowercase() != shaClaim.lowercase()) throw IllegalArgumentException("sha_mismatch expected $shaClaim got $got")
        val prefs = ctx.getSharedPreferences("bridge", 0)
        val treeUriStr = prefs.getString("saf_tree_uri", null)
        val treeUri: Uri? = treeUriStr?.let { try { Uri.parse(it) } catch (_: Exception) { null } }
        val parentRel = rel.substringBeforeLast("/", "")
        val fileName = rel.substringAfterLast("/")
        var written = false
        if (treeUri != null) {
            try {
                val tree = DocumentFile.fromTreeUri(ctx, treeUri)!!
                var parentDoc = tree
                if (parentRel.isNotEmpty()) {
                    for (seg in parentRel.split("/")) {
                        if (seg.isEmpty()) continue
                        var next = parentDoc.findFile(seg)
                        if (next == null) next = parentDoc.createDirectory(seg)
                        if (next != null) parentDoc = next
                    }
                }
                var fileDoc = parentDoc.findFile(fileName)
                if (fileDoc == null) fileDoc = parentDoc.createFile("application/octet-stream", fileName)
                fileDoc?.let { doc ->
                    val mode = if (offset == 0L) "w" else "wa"
                    ctx.contentResolver.openOutputStream(doc.uri, mode)?.use { out ->
                        out.write(bytes)
                        written = true
                    }
                }
            } catch (_: Exception) {}
        }
        if (!written && Build.VERSION.SDK_INT >= 30 && Environment.isExternalStorageManager()) {
            val root = Environment.getExternalStorageDirectory()
            val f = File(root, rel)
            f.parentFile?.mkdirs()
            java.io.RandomAccessFile(f, "rw").use { raf ->
                raf.seek(offset)
                raf.write(bytes)
            }
            written = true
        }
        if (!written) throw IllegalStateException("write failed: no SAF and no File permission")
        return JSONObject().apply {
            put("id", payload.optString("id"))
            put("path", path)
            put("offset", offset)
            put("received", true)
            val sizeOnDisk = try {
                val file = File(Environment.getExternalStorageDirectory(), rel)
                if (file.exists()) file.length() else offset + bytes.size
            } catch (_: Exception) { offset + bytes.size }
            put("sizeOnDisk", sizeOnDisk)
        }
    }

    fun buildTrashRequestUris(ctx: Context, paths: List<String>): List<Uri> {
        val uris = mutableListOf<Uri>()
        val resolver = ctx.contentResolver
        for (p in paths) {
            try {
                val name = p.substringAfterLast("/")
                val cur = resolver.query(MediaStore.Files.getContentUri("external"), arrayOf(MediaStore.MediaColumns._ID), "${MediaStore.MediaColumns.DISPLAY_NAME}=?", arrayOf(name), null)
                cur?.use {
                    if (it.moveToFirst()) {
                        val id = it.getLong(0)
                        uris.add(MediaStore.Files.getContentUri("external", id))
                    }
                }
            } catch (_: Exception) {}
        }
        return uris
    }
}
