import { chromium } from "playwright";
const d=Date.now()+15000; let b;
while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok){b=await chromium.connectOverCDP("http://127.0.0.1:9223");break;}}catch{}await new Promise(r=>setTimeout(r,400));}
if(!b) throw new Error("no cdp");
const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
let page=null;
for(const p of pages){const url=p.url(); if(!url.includes("about:blank") && await p.evaluate(()=>!!window.__TAURI_INTERNALS__).catch(()=>false)){page=p;break;}}
if(!page) throw new Error("no app page");
const res=await page.evaluate(async()=>{
 const inv=window.__TAURI_INTERNALS__.invoke;
 const out={};
 try{out.status=await inv("get_controls_status");}catch(e){out.statusErr=String(e);}
 try{const before=out.status; out.volumeBefore=before?.volumePercent; await inv("control_set_volume",{percent:55}); const after=await inv("get_controls_status"); out.volumeAfter=after.volumePercent;}catch(e){out.volumeErr=String(e);}
 try{const before=await inv("get_controls_status"); await inv("control_set_muted",{muted:!before.muted}); const after=await inv("get_controls_status"); out.muteBefore=before.muted; out.muteAfter=after.muted; await inv("control_set_muted",{muted:before.muted});}catch(e){out.muteErr=String(e);}
 try{const s=await inv("get_controls_status"); out.outputs=s.audioOutputs; if(s.audioOutputs.length){const other=s.audioOutputs.find(d=>!d.isDefault)||s.audioOutputs[0]; out.outputTry=other.name; await inv("control_set_audio_output",{deviceId:other.id}); const after=await inv("get_controls_status"); out.outputAfter=after.defaultAudioId;}}catch(e){out.outputErr=String(e);}
 try{await inv("control_set_wifi",{enabled:true}); out.wifiSet="ok";}catch(e){out.wifiErr=String(e);}
 try{await inv("control_set_bluetooth",{enabled:false}); out.btSet="ok";}catch(e){out.btErr=String(e);}
 try{await inv("control_open_settings",{page:"privacy_location"}); out.openSettings=true;}catch(e){out.openSettingsErr=String(e);}
 return out;
});
const s=res.status||{};
console.log(JSON.stringify({status:{wifi:s.wifi, wifiEnabled:s.wifiEnabled, wifiError:s.wifiError, volume:s.volumePercent, muted:s.muted, outputs:s.audioOutputs, defaultAudioId:s.defaultAudioId, brightness:s.brightnessPercent, brightnessCap:s.brightnessCapability, bt:s.bluetooth, btEnabled:s.bluetoothEnabled, btDevices:s.bluetoothDevices, btError:s.bluetoothError, notificationsCap:s.notifications, notificationItems:s.notificationItems?.length}, volumeBefore:res.volumeBefore, volumeAfter:res.volumeAfter, volumeErr:res.volumeErr, muteBefore:res.muteBefore, muteAfter:res.muteAfter, muteErr:res.muteErr, outputTry:res.outputTry, outputAfter:res.outputAfter, outputErr:res.outputErr, wifiSet:res.wifiSet, wifiErr:res.wifiErr, btSet:res.btSet, btErr:res.btErr, openSettings:res.openSettings, openSettingsErr:res.openSettingsErr, statusErr:res.statusErr},null,1).slice(0,6000));
await b.close().catch(()=>{});
