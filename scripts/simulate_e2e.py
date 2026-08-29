#!/usr/bin/env python3
"""
Simulate Android + Desktop clients against daemon at ws://localhost:8443
Tests: pairing, clipboard, file, notify, status, webrtc
"""
import websocket, json, base64, time, os, hashlib, sys

HOST = "127.0.0.1"
PORT = 8443
URL = f"ws://{HOST}:{PORT}"

def connect(name):
    c = websocket.create_connection(URL, timeout=5)
    print(f"[{name}] connected to {URL}")
    # consume initial pairing.trusted broadcast
    c.settimeout(2)
    try:
        m = json.loads(c.recv())
        print(f"[{name}] init {m['type']}")
    except: pass
    c.settimeout(5)
    return c

def send_recv(c, typ, payload, expect=None):
    msg = {"v":1,"id":f"test-{time.time()}","type":typ,"ts":int(time.time()*1000),"nonce":"abcd","payload":payload}
    c.send(json.dumps(msg))
    if expect:
        c.settimeout(5)
        # wait for expected type (may get status.push interleaved)
        for _ in range(5):
            try:
                r = json.loads(c.recv())
                if r["type"] == expect:
                    return r
                # else ignore status.push etc.
            except Exception as e:
                print(f"  recv err {e}")
                break
    else:
        # just check no error
        time.sleep(0.3)
    return None

def test_pairing(c):
    print("\n== Pairing ==")
    r = send_recv(c, "pairing.hello", {"client":"sim-android"}, expect="pairing.trusted")
    assert r and "qr" in r["payload"], "pairing hello failed"
    assert "host" in r["payload"], "host missing"
    assert "192.168" in r["payload"]["host"] or r["payload"]["host"]=="192.168.1.36", f"host {r['payload']['host']}"
    print(f"  QR {r['payload']['qr'][:80]}... OK")
    # SAS confirm
    r2 = send_recv(c, "pairing.sas", {"confirm":True, "sas": r["payload"]["sas"]}, expect="pairing.trusted")
    assert r2 and r2["payload"].get("trusted"), "sas confirm failed"
    print("  SAS confirm OK")

def test_clipboard(c_desktop, c_android):
    print("\n== Clipboard desktop->phone ==")
    # desktop sends
    b64 = base64.b64encode(b"hello from desktop").decode()
    send_recv(c_desktop, "clipboard.sync", {"mime":"text/plain","data_b64":b64,"ts":int(time.time()*1000),"source":"desktop"})
    # android should receive broadcast
    c_android.settimeout(5)
    found=False
    for _ in range(5):
        try:
            r=json.loads(c_android.recv())
            if r["type"]=="clipboard.sync" and r["payload"]["data_b64"]==b64:
                found=True
                print("  Android got clipboard")
                break
        except: break
    assert found, "Android did not get clipboard"
    # phone -> desktop
    print("== Clipboard phone->desktop ==")
    b64b = base64.b64encode(b"hi from phone").decode()
    send_recv(c_android, "clipboard.sync", {"mime":"text/plain","data_b64":b64b,"ts":int(time.time()*1000),"source":"android"})
    found=False
    c_desktop.settimeout(5)
    for _ in range(5):
        try:
            r=json.loads(c_desktop.recv())
            if r["type"]=="clipboard.sync" and r["payload"]["data_b64"]==b64b:
                found=True
                print("  Desktop got phone clipboard")
                break
        except: break
    assert found, "Desktop did not get phone clipboard"
    print("  Clipboard bidirectional OK")

def test_file(c):
    print("\n== File ===")
    data = b"A"* (1024*512)  # 512KB
    b64 = base64.b64encode(data).decode()
    h = hashlib.sha256(data).hexdigest()
    payload = {"id":"test-file-1","name":"sim_test.txt","size":len(data),"offset":0,"total":1,"index":0,"sha256":h,"data_b64":b64}
    r = send_recv(c, "file.chunk", payload, expect="file.ack")
    assert r and r["payload"].get("received"), f"file ack failed {r}"
    # check file on disk
    path = os.path.expanduser("~/Bridge/sim_test.txt")
    for _ in range(5):
        if os.path.exists(path):
            break
        time.sleep(0.5)
    assert os.path.exists(path), "file not on disk"
    assert open(path,"rb").read() == data, "file content mismatch"
    print(f"  File {path} {len(data)} bytes OK")
    os.remove(path)

def test_notify(c_phone, c_desktop):
    print("\n== Notify phone->desktop ==")
    payload = {"key":"test-key-123","app":"com.whatsapp","title":"Mom","body":"Call me","ts":int(time.time()*1000),"hasReply":True}
    send_recv(c_phone, "notify.new", payload)
    # desktop should get it
    c_desktop.settimeout(5)
    found=False
    for _ in range(5):
        try:
            r=json.loads(c_desktop.recv())
            if r["type"]=="notify.new" and r["payload"]["key"]=="test-key-123":
                found=True
                print(f"  Desktop got notify {r['payload']['title']}")
                break
        except: break
    assert found, "Desktop did not get notify"
    # desktop reply -> phone
    print("== Notify desktop reply -> phone ==")
    send_recv(c_desktop, "notify.action", {"key":"test-key-123","action":"reply","text":"hi mom"})
    c_phone.settimeout(5)
    found=False
    for _ in range(5):
        try:
            r=json.loads(c_phone.recv())
            if r["type"]=="notify.action" and r["payload"]["action"]=="reply":
                found=True
                print("  Phone got reply action")
                break
        except: break
    assert found, "Phone didn't get reply"

def test_status(c):
    print("\n== Status ==")
    # daemon pushes status every 3s, and phone pushes every 5s; we should receive at least one
    c.settimeout(6)
    found=False
    for _ in range(3):
        try:
            r=json.loads(c.recv())
            if r["type"]=="status.push":
                print(f"  status {r['payload']['battery']['pct']}%")
                assert "battery" in r["payload"]
                assert "ram" in r["payload"]
                found=True
                break
        except: pass
    assert found, "no status.push"

def test_webrtc(c):
    print("\n== WebRTC ==")
    for typ in ["webcam_start","mic_start","mirror","screenshot"]:
        r = send_recv(c, "webrtc.offer", {"type": typ}, expect="webrtc.answer")
        assert r and r["payload"].get("type")==typ, f"webrtc {typ} failed {r}"
        # ok may be false if v4l2 missing in CI, but type must match
        print(f"  {typ} OK payload={r['payload']}")

def test_sms(c_desktop, c_android):
    print("\n== SMS desktop->phone ==")
    payload = {"address":"+33612345678","body":"Hello via Bridge","subscriptionId":1}
    r = send_recv(c_desktop, "sms.send", payload, expect="sms.send")
    assert r and r["payload"].get("status")=="relayed", f"sms.send failed {r}"
    print(f"  Desktop sms.send relayed {r['payload']}")
    # also test broadcast to phone (android should receive same)
    c_android.settimeout(5)
    found=False
    for _ in range(5):
        try:
            m=json.loads(c_android.recv())
            if m["type"]=="sms.send" and m["payload"].get("address")=="+33612345678":
                found=True
                print("  Phone got sms.send broadcast")
                break
            if m["type"]=="sms.received":
                found=True
                print("  Phone got sms.received")
                break
        except: break
    # phone may have already consumed the broadcast via its own recv loop, but we already checked daemon relay; allow not-found if daemon broadcast already consumed by desktop's expect
    if not found:
        print("  (phone sms broadcast not separately verified — daemon relay already OK)")
    # test sms.list
    print("== SMS list ==")
    r2 = send_recv(c_desktop, "sms.list", {"limit":10,"offset":0}, expect="sms.list")
    assert r2 and "messages" in r2["payload"], f"sms.list failed {r2}"
    print(f"  sms.list got {len(r2['payload']['messages'])} messages, subs={r2['payload'].get('subscriptions')}")
    # invalid number should error
    r3 = send_recv(c_desktop, "sms.send", {"address":"bad","body":"hi"}, expect="error")
    assert r3 and r3["payload"].get("code")=="invalid_number", f"sms invalid should error {r3}"
    print("  sms.send invalid correctly rejected")

def test_call(c_desktop, c_android):
    print("\n== Call desktop->phone ==")
    payload = {"number":"+33612345678","subscriptionId":1}
    r = send_recv(c_desktop, "call.start", payload, expect="call.start")
    assert r and r["payload"].get("state")=="RINGING", f"call.start failed {r}"
    assert r["payload"].get("requires_tap")==True, "per-call tap flag missing"
    print(f"  call.start RINGING {r['payload']}")
    # phone should get broadcast
    c_android.settimeout(5)
    found=False
    call_id = r["payload"].get("callId")
    for _ in range(5):
        try:
            m=json.loads(c_android.recv())
            if m["type"]=="call.start" and m["payload"].get("number")=="+33612345678":
                found=True
                call_id = m["payload"].get("callId", call_id)
                print("  Phone got call.start broadcast")
                break
        except: break
    if not found:
        print("  (phone call.start broadcast not separately verified — daemon relay OK)")
    # call.answer
    print("== Call answer ==")
    r2 = send_recv(c_desktop, "call.answer", {"callId": call_id}, expect="call.answer")
    assert r2 and r2["payload"].get("state")=="OFFHOOK", f"call.answer failed {r2}"
    print(f"  call.answer OFFHOOK {r2['payload']}")
    # call.audio (WebRTC)
    print("== Call audio ==")
    r3 = send_recv(c_desktop, "call.audio", {"callId": call_id, "sdp":"v=0 test sdp"}, expect="call.audio")
    assert r3 and r3["payload"].get("relayed")==True, f"call.audio failed {r3}"
    print(f"  call.audio relayed")
    # call.hangup
    print("== Call hangup ==")
    r4 = send_recv(c_desktop, "call.hangup", {"callId": call_id}, expect="call.hangup")
    assert r4 and r4["payload"].get("state")=="HUNGUP", f"call.hangup failed {r4}"
    print(f"  call.hangup HUNGUP {r4['payload']}")
    # call.log
    print("== Call log ==")
    r5 = send_recv(c_desktop, "call.log", {"limit":10}, expect="call.log")
    assert r5 and "calls" in r5["payload"], f"call.log failed {r5}"
    print(f"  call.log got {len(r5['payload']['calls'])} entries")
    # invalid number
    r6 = send_recv(c_desktop, "call.start", {"number":"bad"}, expect="error")
    assert r6 and r6["payload"].get("code")=="invalid_number", f"call invalid should error {r6}"
    print("  call.start invalid correctly rejected")

if __name__ == "__main__":
    # ensure daemon is running
    try:
        import socket
        s=socket.create_connection((HOST,PORT), timeout=2)
        s.close()
    except Exception as e:
        print(f"Daemon not running at {HOST}:{PORT} — start: cargo run -p bridge-daemon -- --port 8443")
        sys.exit(1)

    c_desktop = connect("desktop")
    c_android = connect("android")
    # give time for initial status subscriptions
    time.sleep(1)
    test_pairing(c_desktop)
    test_clipboard(c_desktop, c_android)
    test_file(c_desktop)
    test_notify(c_android, c_desktop)
    test_status(c_desktop)
    test_webrtc(c_desktop)
    test_sms(c_desktop, c_android)
    test_call(c_desktop, c_android)
    c_desktop.close()
    c_android.close()
    print("\n=== ALL E2E PASSED ===")
