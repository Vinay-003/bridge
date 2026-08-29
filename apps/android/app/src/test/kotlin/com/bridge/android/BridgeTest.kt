package com.bridge.android

import org.junit.Test
import org.junit.Assert.*
import java.util.Base64

class BridgeTest {
    @Test fun pairingParse() {
        val qr = "bridge://pair?v=1&id=abc&host=192.168.1.36&ecdh=pub&fp=fp123&port=8443"
        // Parse without android.net.Uri (use java.net.URI)
        val uri = java.net.URI(qr)
        val query = uri.query.split("&").associate { it.split("=")[0] to it.split("=")[1] }
        assertEquals("abc", query["id"])
        assertEquals("192.168.1.36", query["host"])
        assertEquals("8443", query["port"])
    }
    @Test fun clipboardBase64() {
        val text = "hello"
        val b64 = Base64.getEncoder().encodeToString(text.toByteArray())
        val decoded = String(Base64.getDecoder().decode(b64))
        assertEquals(text, decoded)
    }
    @Test fun pairingWithHost() {
        val qr = "bridge://pair?v=1&id=id1&host=10.0.0.5&ecdh=a&fp=fp&port=8443"
        assertTrue(qr.contains("host=10.0.0.5"))
    }
}
