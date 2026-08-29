// Bridge WebSocket client — mirrors protocol in bridge-core
export type BridgeMessage = {
  v: number; id: string; type: string; ts: number; nonce: string; payload: any;
}
export class BridgeClient {
  ws: WebSocket | null = null;
  url: string;
  handlers: Map<string, ((m: BridgeMessage)=>void)[]> = new Map();
  onState: (s: "disconnected"|"connecting"|"connected")=>void = ()=>{};
  constructor(url = `ws://${location.hostname}:8443`) { this.url = url; }
  connect() {
    this.onState("connecting");
    try {
      this.ws = new WebSocket(this.url);
      this.ws.onopen = ()=> this.onState("connected");
      this.ws.onclose = ()=> { this.onState("disconnected"); setTimeout(()=>this.connect(), 3000); };
      this.ws.onerror = ()=> this.onState("disconnected");
      this.ws.onmessage = (e)=>{
        try {
          const m = JSON.parse(e.data) as BridgeMessage;
          const lst = this.handlers.get(m.type) || [];
          lst.forEach(fn=>fn(m));
          (this.handlers.get("*")||[]).forEach(fn=>fn(m));
        } catch {}
      };
    } catch { this.onState("disconnected"); }
  }
  on(type: string, fn:(m:BridgeMessage)=>void) {
    if(!this.handlers.has(type)) this.handlers.set(type, []);
    this.handlers.get(type)!.push(fn);
  }
  send(type: string, payload:any) {
    const msg: BridgeMessage = { v:1, id: crypto.randomUUID(), type, ts: Date.now(), nonce: Math.random().toString(16).slice(2,10), payload };
    this.ws?.send(JSON.stringify(msg));
    return msg;
  }
}
export const bridge = new BridgeClient();
if(typeof window!=="undefined") bridge.connect();
