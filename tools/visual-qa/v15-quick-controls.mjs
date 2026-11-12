import { chromium } from "playwright";
const d=Date.now()+15000; let b;
while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok){b=await chromium.connectOverCDP("http://127.0.0.1:9223");break;}}catch{}await new Promise(r=>setTimeout(r,400));}
if(!b) throw new Error("no cdp");
const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
let page=null;for(const p of pages){const k=await p.evaluate(()=>document.documentElement.dataset.window).catch(()=>null); if(k==="island") page=p;}
if(!page) page=pages[0];
const res=await page.evaluate(async()=>{
 const inv=window.__TAURI_INTERNALS__.invoke;
 const out={};
 try{out.status=await inv("get_controls_status");}catch(e){out.statusErr=String(e);}
 try{const before=await inv("get_controls_status"); out.volumeBefore=before.volumePercent; await inv("control_set_volume",{percent:55}); const after=await inv("get_controls_status"); out.volumeAfter=after.volumePercent;}catch(e){out.volumeErr=String(e);}
 try{const before=await inv("get_controls_status"); await inv("control_set_muted",{muted:!before.muted}); const after=await inv("get_controls_status"); out.muteBefore=before.muted; out.muteAfter=after.muted; await inv("control_set_muted",{muted:before.muted});}catch(e){out.muteErr=String(e);}
 try{const s=await inv("get_controls_status"); if(s.audioOutputs.length){const other=s.audioOutputs.find(d=>!d.isDefault)||s.audioOutputs[0]; out.outputTry=other.name; await inv("control_set_audio_output",{deviceId:other.id}); const after=await inv("get_controls_status"); out.outputAfter=after.defaultAudioId;}}catch(e){out.outputErr=String(e);}
 try{await inv("control_set_wifi",{enabled:true});}catch(e){out.wifiErr=String(e);}
 try{await inv("control_set_bluetooth",{enabled:false});}catch(e){out.btErr=String(e);}
 try{await inv("control_open_settings",{page:"privacy_location"}); out.openSettings=true;}catch(e){out.openSettingsErr=String(e);}
 return out;
});
const red=(o)=>{ if(o&&o.status) return {wifi:o.status.wifi, wifiEnabled:o.status.wifiEnabled, wifiError:o.status.wifiError, volume:o.status.volumePercent, muted:o.status.muted, audioOutputs:o.status.audioOutputs, brightness:o.status.brightnessPercent, brightnessCap:o.status.brightnessCapability, bt:o.status.bluetooth, btEnabled:o.status.bluetoothEnabled, btDevices:o.status.bluetoothDevices, btError:o.status.bluetoothError, notifications:o.status.notifications}; return o;};
console.log(JSON.stringify(red(res),null,1).slice(0,4000));
await b.close().catch(()=>{});
