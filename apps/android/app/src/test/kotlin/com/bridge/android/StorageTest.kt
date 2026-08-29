package com.bridge.android

import com.bridge.android.storage.StorageHandler
import org.junit.Test
import org.junit.Assert.*

class StorageTest {

    @Test fun sanitizePhonePath_ok() {
        assertEquals("Photos/img.jpg", StorageHandler.sanitizePhonePath("/Photos/img.jpg"))
        assertEquals("", StorageHandler.sanitizePhonePath("/"))
        assertEquals("report.pdf", StorageHandler.sanitizePhonePath("report.pdf"))
    }

    @Test fun sanitizePhonePath_rejectsTraversal() {
        try { StorageHandler.sanitizePhonePath("../secret"); fail("should throw") } catch(_:Exception){}
        try { StorageHandler.sanitizePhonePath("/a/../../etc"); fail("should throw") } catch(_:Exception){}
        try { StorageHandler.sanitizePhonePath(""); fail("should throw") } catch(_:Exception){}
        try { StorageHandler.sanitizePhonePath("/\u0000bad"); fail("should throw") } catch(_:Exception){}
    }

    @Test fun validateLs_ok() {
        assertTrue(StorageHandler.validateLs(mapOf("path" to "/")).isSuccess)
        assertTrue(StorageHandler.validateLs(mapOf("path" to "/DCIM")).isSuccess)
    }

    @Test fun validateLs_rejects() {
        assertTrue(StorageHandler.validateLs(mapOf("path" to "../escape")).isFailure)
        assertTrue(StorageHandler.validateLs(mapOf("path" to "")).isFailure)
        assertTrue(StorageHandler.validateLs(emptyMap()).isFailure)
    }

    @Test fun validateSync_sha() {
        val ok = mapOf("id" to "u", "path" to "/a.bin", "size" to 1024L, "offset" to 0L, "total" to 1, "index" to 0, "sha256" to "a".repeat(64), "data_b64" to "")
        assertTrue(StorageHandler.validateSync(ok).isSuccess)
        val bad = mapOf("id" to "u", "path" to "/a.bin", "size" to 1024L, "offset" to 0L, "total" to 1, "index" to 0, "sha256" to "bad", "data_b64" to "")
        assertTrue(StorageHandler.validateSync(bad).isFailure)
    }

    @Test fun chunkMath_4gb() {
        val chunkSize = 1_048_576L
        val offset = 3_221_225_472L // 3072 * 1MB
        val size = 5_000_000_000L
        assertTrue(offset < size)
        val idx = (offset / chunkSize).toInt()
        assertEquals(3072, idx)
        assertEquals(offset, idx * chunkSize)
    }

    @Test fun vectorClock_dominates() {
        val a = mapOf("daemon" to 3L, "phone" to 2L)
        val b = mapOf("daemon" to 2L, "phone" to 2L)
        assertTrue(StorageHandler.dominatesVector(a,b))
        assertFalse(StorageHandler.dominatesVector(b,a))
    }

    @Test fun vectorClock_concurrent() {
        val a = mapOf("daemon" to 3L, "phone" to 1L)
        val b = mapOf("daemon" to 2L, "phone" to 2L)
        assertTrue(StorageHandler.isConcurrent(a,b))
        assertFalse(StorageHandler.isConcurrent(mapOf("daemon" to 3L, "phone" to 2L), mapOf("daemon" to 2L, "phone" to 2L)))
    }

    @Test fun stateMachine_valid() {
        assertTrue(StorageHandler.StorageState.IDLE.canTransition(StorageHandler.StorageState.SCANNING))
        assertTrue(StorageHandler.StorageState.SCANNING.canTransition(StorageHandler.StorageState.SYNCING))
        assertTrue(StorageHandler.StorageState.SYNCING.canTransition(StorageHandler.StorageState.CONFLICT))
        assertTrue(StorageHandler.StorageState.CONFLICT.canTransition(StorageHandler.StorageState.SYNCING))
        assertTrue(StorageHandler.StorageState.SYNCING.canTransition(StorageHandler.StorageState.DONE))
        assertTrue(StorageHandler.StorageState.DONE.canTransition(StorageHandler.StorageState.IDLE))
    }

    @Test fun stateMachine_invalid() {
        assertFalse(StorageHandler.StorageState.IDLE.canTransition(StorageHandler.StorageState.CONFLICT))
        assertFalse(StorageHandler.StorageState.IDLE.canTransition(StorageHandler.StorageState.SYNCING))
        assertFalse(StorageHandler.StorageState.DONE.canTransition(StorageHandler.StorageState.SCANNING))
    }

    @Test fun trashInfo_format() {
        val info = StorageHandler.formatTrashInfo("/sdcard/Bridge/old.pdf", "2026-08-13T12:00:00Z")
        assertTrue(info.contains("Path=/sdcard/Bridge/old.pdf"))
        assertTrue(info.contains("DeletionDate="))
        val parsed = StorageHandler.parseTrashInfo(info)
        assertEquals("/sdcard/Bridge/old.pdf", parsed["Path"])
    }
}
