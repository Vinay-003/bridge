export type BridgeMessage = {
  v: number; id: string; type: string; ts: number; nonce: string; payload: any;
}
export class BridgeClient {
  ws: WebSocket | null = null;
  url: string;
  urls: string[];
  handlers: Map<string, ((m: BridgeMessage)=>void)[]> = new Map();
  onState: (s: "disconnected"|"connecting"|"connected")=>void = ()=>{};
  private tryIdx = 0;
  constructor(url?: string) {
    const host = location.hostname;
    const base = url || `ws://${host}:8443`;
    // try localhost fallback if host is not 127.0.0.1
    this.urls = [base];
    if (host !== "127.0.0.1" && host !== "localhost") {
      this.urls.push(`ws://127.0.0.1:8443`);
      this.urls.push(`ws://localhost:8443`);
    }
    this.url = this.urls[0];
  }
  connect() {
    const tryUrl = this.urls[this.tryIdx % this.urls.length];
    this.url = tryUrl;
    this.onState("connecting");
    try {
      if (this.ws) { try { this.ws.close(); } catch {} }
      this.ws = new WebSocket(tryUrl);
      this.ws.onopen = ()=> { this.tryIdx = 0; this.onState("connected"); this.send("pairing.hello", { client: "desktop" }); };
      this.ws.onclose = ()=> { this.onState("disconnected"); this.tryIdx++; setTimeout(()=>this.connect(), 2000); };
      this.ws.onerror = ()=> { this.onState("disconnected"); };
      this.ws.onmessage = (e)=>{
        try {
          const m = JSON.parse(e.data) as BridgeMessage;
          const lst = this.handlers.get(m.type) || [];
          lst.forEach(fn=>fn(m));
          (this.handlers.get("*")||[]).forEach(fn=>fn(m));
        } catch {}
      };
    } catch { this.onState("disconnected"); setTimeout(()=>this.connect(), 2000); }
  }
  on(type: string, fn:(m:BridgeMessage)=>void) {
    if(!this.handlers.has(type)) this.handlers.set(type, []);
    this.handlers.get(type)!.push(fn);
  }
  off(type: string, fn:(m:BridgeMessage)=>void) {
    const arr = this.handlers.get(type);
    if(arr) { const i=arr.indexOf(fn); if(i>=0) arr.splice(i,1); }
  }
  send(type: string, payload:any) {
    const msg: BridgeMessage = { v:1, id: crypto.randomUUID(), type, ts: Date.now(), nonce: Math.random().toString(16).slice(2,10), payload };
    if (this.ws && this.ws.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(msg));
    return msg;
  }
}
export const bridge = new BridgeClient();
const isVitest = typeof process !== "undefined" && !!process.env.VITEST;
if(typeof window!=="undefined" && !isVitest) {
  // defer connect to allow onState to be set by React
  setTimeout(()=>bridge.connect(), 100);
}
