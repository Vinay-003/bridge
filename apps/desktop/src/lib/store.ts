import {create} from 'zustand'

export type DeviceStatus = {
  battery:{pct:number,charging:boolean,tempC:number};
  ram:{availMb:number,totalMb:number};
  storage:{freeGb:number,totalGb:number};
  signal:{dbm:number,bars:number};
}
export type Notif = { key:string; app:string; title:string; body:string; ts:number; hasReply:boolean; }
export type SmsMessage = { id:string; address:string; body:string; date:number; type:number; read:number; subscriptionId?:number }
export type CallEntry = { number:string; type:string; date:number; duration:number; subscriptionId?:number }
export type CallState = "IDLE"|"RINGING"|"OFFHOOK"|"HUNGUP"
type S = {
  connected:boolean;
  pairing:{ qr:string; fp:string; sas:string } | null;
  status: DeviceStatus | null;
  notifs: Notif[];
  clipboard: string;
  transfers: { id:string; name:string; pct:number; done:boolean }[];
  sms: SmsMessage[];
  calls: CallEntry[];
  callState: CallState;
  activeCall: { id:string; number:string; state:CallState } | null;
  set: (p:Partial<S>)=>void;
}
export const useBridgeStore = create<S>((set)=>({
  connected:false,
  pairing:null,
  status:null,
  notifs:[],
  clipboard:"",
  transfers:[],
  sms:[],
  calls:[],
  callState:"IDLE",
  activeCall:null,
  set:(p)=>set(p as any),
}))
