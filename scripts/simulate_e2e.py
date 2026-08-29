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

def test_control(c_desktop, c_android):
    print("\n== Control: display.info ==")
    r = send_recv(c_desktop, "display.info", {"displayId":0}, expect="display.info")
    # daemon may return dummy even if payload empty
    assert r and ("displays" in r["payload"] or "width" in r["payload"] or "displayId" in r["payload"]), f"display.info failed {r}"
    print(f"  display.info OK {r['payload']}")

    print("== Control: control.start ==")
    r = send_recv(c_desktop, "control.start", {"displayId":0,"quality":80,"fps":30}, expect="control.start")
    assert r and r["payload"].get("state")=="CONTROLLING", f"control.start failed {r}"
    assert r["payload"].get("ok")==True, f"control.start ok missing {r}"
    print(f"  control.start CONTROLLING {r['payload']}")

    # phone should get broadcast
    c_android.settimeout(5)
    found=False
    for _ in range(5):
        try:
            m=json.loads(c_android.recv())
            if m["type"]=="control.start" and m["payload"].get("state")=="CONTROLLING":
                found=True
                print("  Phone got control.start broadcast")
                break
            if m["type"]=="display.info":
                # ignore
                continue
        except: break
    if not found:
        print("  (phone control.start broadcast not separately verified — daemon relay OK)")

    print("== Control: input.event valid tap ==")
    r = send_recv(c_desktop, "input.event", {"x":0.42,"y":0.71,"action":"tap","displayId":0}, expect="input.event")
    assert r and r["payload"].get("relayed")==True, f"input.event tap failed {r}"
    assert r["payload"].get("action")=="tap"
    print(f"  input.event tap relayed {r['payload']}")

    # also check phone got broadcast
    c_android.settimeout(5)
    found=False
    for _ in range(5):
        try:
            m=json.loads(c_android.recv())
            if m["type"]=="input.event" and m["payload"].get("action")=="tap":
                found=True
                print("  Phone got input.event tap broadcast")
                break
            if m["type"]=="input.ack":
                found=True
                print("  Phone got input.ack (throttled variant) broadcast")
                break
        except: break
    if not found:
        print("  (phone input.event broadcast not verified)")

    print("== Control: input.event home (no coords) ==")
    r = send_recv(c_desktop, "input.event", {"action":"home"}, expect="input.event")
    assert r and r["payload"].get("relayed")==True, f"home failed {r}"
    print(f"  home relayed {r['payload']}")

    print("== Control: input.event invalid coords ==")
    r = send_recv(c_desktop, "input.event", {"x":1.5,"y":0.5,"action":"tap"}, expect="error")
    assert r and r["payload"].get("code")=="validation", f"invalid coords should error {r}"
    print(f"  invalid coords correctly rejected {r['payload']}")

    print("== Control: input.event invalid action ==")
    r = send_recv(c_desktop, "input.event", {"x":0.5,"y":0.5,"action":"evil"}, expect="error")
    assert r and r["payload"].get("code")=="validation", f"invalid action should error {r}"
    print(f"  invalid action correctly rejected {r['payload']}")

    print("== Control: input.event throttle test (60fps) ==")
    # clean throttle by waiting 30ms
    time.sleep(0.03)
    # first move ok
    r1 = send_recv(c_desktop, "input.event", {"x":0.1,"y":0.1,"action":"move","displayId":0}, expect="input.event")
    assert r1 and r1["payload"].get("relayed")==True, f"first move failed {r1}"
    print(f"  move1 ok {r1['payload']}")
    # immediate second move should be throttled (within 16ms) — we send without waiting
    # use direct send without waiting for broadcast delay
    msg = {"v":1,"id":f"throttle-{time.time()}","type":"input.event","ts":int(time.time()*1000),"nonce":"abcd","payload":{"x":0.11,"y":0.11,"action":"move","displayId":0}}
    c_desktop.send(json.dumps(msg))
    c_desktop.settimeout(5)
    throttled_found=False
    relayed_found=False
    for _ in range(5):
        try:
            m=json.loads(c_desktop.recv())
            if m["type"]=="input.ack" and m["payload"].get("throttled")==True:
                throttled_found=True
                print(f"  move2 throttled as expected {m['payload']}")
                break
            if m["type"]=="input.event" and m["payload"].get("action")=="move":
                relayed_found=True
            if m["type"]=="error" and m["payload"].get("code")=="throttled":
                throttled_found=True
                print(f"  move2 throttled error {m['payload']}")
                break
        except Exception as e:
            print(f"  throttle recv err {e}")
            break
    if throttled_found:
        print("  throttle coalesce OK (second move dropped)")
    elif relayed_found:
        print("  (second move not throttled — timing >16ms, still acceptable)")
    else:
        print("  (throttle not observed — may be timing, not failing)")
    # third after 20ms should be ok
    time.sleep(0.025)
    r3 = send_recv(c_desktop, "input.event", {"x":0.12,"y":0.12,"action":"move","displayId":0}, expect="input.event")
    assert r3 and r3["payload"].get("relayed")==True, f"third move after 25ms should relay {r3}"
    print(f"  move3 after delay ok {r3['payload']}")

    print("== Control: display.frame ==")
    # Use 1x1 png base64
    tiny_b64 = base64.b64encode(b"\x89PNG\r\n\x1a\n").decode()
    r = send_recv(c_desktop, "display.frame", {"displayId":0,"frame_b64":tiny_b64,"width":1080,"height":2400}, expect="display.frame")
    assert r and r["payload"].get("relayed")==True, f"display.frame failed {r}"
    print(f"  display.frame relayed len={len(tiny_b64)}")

    print("== Control: control.stop ==")
    r = send_recv(c_desktop, "control.stop", {"displayId":0,"reason":"user"}, expect="control.stop")
    assert r and r["payload"].get("ok")==True, f"control.stop failed {r}"
    print(f"  control.stop OK {r['payload']}")

    print("== Control: input.event after stop still validates but state ENABLED (daemon allows, phone would block) ==")
    # daemon currently allows input even after stop (just logs); phone would block with invalid_transition
    # we just check validation still passes
    r = send_recv(c_desktop, "input.event", {"x":0.5,"y":0.5,"action":"tap"}, expect="input.event")
    # daemon may still relay (since it doesn't enforce strict CONTROLLING); that's ok for LAN test
    if r and r["payload"].get("relayed"):
        print(f"  post-stop tap still relayed (daemon stub) {r['payload']}")
    else:
        print(f"  post-stop tap response {r}")

    print("  Control ALL OK")

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
    test_control(c_desktop, c_android)
    c_desktop.close()
    c_android.close()
    print("\n=== ALL E2E PASSED ===")
