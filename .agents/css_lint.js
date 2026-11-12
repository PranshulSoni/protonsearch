const fs=require("fs");
const path=require("path");
const files=["apps/desktop/src/base.css","apps/desktop/src/tokens.css","apps/desktop/src/shell.css","apps/desktop/src/launcher/launcher.css","apps/desktop/src/onboarding/onboarding.css","apps/desktop/src/island/island.css","apps/desktop/src/app/desktop.css","apps/desktop/src/app/ui.css"];
(async()=>{
 let lc; try{lc=require("lightningcss");}catch(e){console.log("no lightningcss",e.message);return;}
 for(const f of files){
  if(!fs.existsSync(f)) continue;
  try{ lc.transform({filename:f,code:fs.readFileSync(f),minify:true}); }
  catch(e){ console.log("ERROR in",f,JSON.stringify({message:e.message,loc:e.loc,line:e.loc&&e.loc.line,column:e.loc&&e.loc.column}).slice(0,800)); }
 }
})();
