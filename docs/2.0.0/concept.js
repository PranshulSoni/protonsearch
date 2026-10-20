"use strict";
(() => {
 const stage = document.getElementById("stage");
 if (!stage) return;
 if (new URLSearchParams(location.search).get("embed") === "1") document.documentElement.classList.add("embed");
 const panel = document.getElementById("widget-panel");
 const heading = document.getElementById("panel-title");
 const content = document.getElementById("panel-content");
 const status = document.getElementById("demo-status");
 const shelf = document.getElementById("dock-shelf");
 const weather = document.getElementById("weather-widget");
 const appGroup = document.getElementById("app-group");
 const controls = Object.fromEntries(["preset","wallpaper","material","profile"].map(id => [id, document.getElementById(id)]));
 const state = { preset:"balanced",wallpaper:"dawn",material:"light",profile:"work",weatherSize:"wide",weatherEnabled:true,joined:false,volume:64,jobStopped:false };
 const apps = { files:"Files",browser:"Browser",code:"Code editor",terminal:"Terminal" };
 let opened = null, invoker = null, editSnapshot = null, orderSnapshot = [];
 const icon = (name, cls = "") => '<svg class="icon ' + cls + '" viewBox="0 0 24 24" aria-hidden="true"><use href="#i-' + name + '"></use></svg>';
 const appImage = name => '<img src="assets/' + name + '.svg" alt="">';
 const row = (name, subtitle) => '<button class="result-row" data-open="' + name + '">' + appImage(name) + '<span><b>' + apps[name] + '</b><small>' + subtitle + '</small></span>' + icon("arrow") + '</button>';
 function announce(message) { status.textContent = message; }
 function apply() {
  ["preset","wallpaper","material","profile"].forEach(key => { stage.dataset[key] = state[key]; controls[key].value = state[key]; });
  weather.dataset.size = state.weatherSize;
  weather.hidden = !state.weatherEnabled;
  stage.querySelector(".weather-divider").hidden = !state.weatherEnabled;
  shelf.classList.toggle("joined", state.joined);
  document.getElementById("profile-indicator").textContent = state.profile[0].toUpperCase() + state.profile.slice(1);
  document.getElementById("mode-indicator").hidden = state.profile !== "presentation";
 }
 function restoreDraft() {
  if (!editSnapshot) return;
  Object.assign(state, editSnapshot);
  orderSnapshot.forEach(node => appGroup.appendChild(node));
  editSnapshot = null;
  stage.classList.remove("editing");
  document.getElementById("edit-cue").hidden = true;
  apply();
 }
 function close(restoreFocus = true) {
  restoreDraft();
  panel.hidden = true;
  stage.querySelectorAll("[aria-expanded]").forEach(item => item.setAttribute("aria-expanded","false"));
  if (restoreFocus && invoker && !invoker.hidden && invoker.isConnected) invoker.focus();
  invoker = null; opened = null;
  announce("Panel closed.");
 }
 function position() {
  if (panel.hidden) return;
  const bounds = stage.getBoundingClientRect();
  const anchor = invoker && stage.contains(invoker) ? invoker.getBoundingClientRect() : shelf.getBoundingClientRect();
  const usableTop = Math.max(170, anchor.top - bounds.top - 24);
  panel.style.maxHeight = Math.min(usableTop, bounds.height - 32) + "px";
  const width = panel.offsetWidth;
  const left = Math.max(16, Math.min(anchor.left - bounds.left + anchor.width / 2 - width / 2, bounds.width - width - 16));
  panel.style.left = left + "px";
  const bottom = Math.max(16, bounds.bottom - anchor.top + 12);
  panel.style.bottom = Math.min(bottom, bounds.height - 190) + "px";
 }
 const templates = {
  weather: () => '<p class="panel-kicker">Pune, India</p><div class="weather-hero"><div><div class="temperature">26°</div><p class="condition">Clear skies · H: 28° L: 21°</p></div>' + icon("sun","sun") + '</div><div class="hourly">' + ["Now","10 AM","11 AM","12 PM","1 PM"].map((t,i) => '<div><small>' + t + '</small>' + icon(i===4?"cloud":"sun",i===4?"":"sun") + '<span>' + [26,27,28,28,27][i] + '°</span></div>').join("") + '</div><div class="forecast-row"><span>Tomorrow</span>' + icon("sun","sun") + '<span>28° / 21°</span></div><div class="forecast-row"><span>Thursday</span>' + icon("cloud") + '<span>25° / 20°</span></div><p class="panel-footnote">Illustrative forecast · city and size are customizable.</p>',
  search: () => '<label class="sr-only" for="sample-search">Search the sample catalog</label><div class="search-field">' + icon("search") + '<input id="sample-search" type="search" placeholder="Apps, files, a thought…" autocomplete="off"></div><div class="result-list" id="search-results"></div><p class="panel-footnote">Sample catalog · select a result to preview it.</p>',
  start: () => '<p class="panel-kicker">A place for everything you open.</p><div class="start-grid">' + Object.keys(apps).map(id => '<button class="start-app" data-open="' + id + '">' + appImage(id) + '<span>' + apps[id] + '</span></button>').join("") + '</div><p class="panel-subheading">Your workspace</p><button class="result-row" data-open="files">' + appImage("files") + '<span><b>' + (state.profile==="presentation"?"Private workspace":"Proton 2.0") + '</b><small>' + (state.profile==="presentation"?"Recent items hidden":"Recent project · Documents") + '</small></span>' + icon("arrow") + '</button><label class="edit-field" for="panel-profile">Profile</label><select class="profile-select" id="panel-profile"><option value="work">Work</option><option value="home">Home</option><option value="presentation">Presentation</option></select><button class="panel-action" data-open="customize">' + icon("settings") + 'Customize dock</button>',
  agents: () => state.profile==="presentation" ? '<p class="condition">Task previews are hidden in Presentation.</p><p class="panel-footnote">Existing tasks keep their permissions and state.</p>' : '<p class="panel-kicker">Your agents, a glance away.</p><div class="job"><b>Draft release notes</b><small>' + (state.jobStopped?"Cancelled in this preview":"Working · local provider") + '</small>' + (state.jobStopped?"":'<div class="job-track"><span></span></div>') + '</div><div class="job"><b>Organize project files</b><small><span class="approval-dot"></span>Waiting for your approval</small></div><button class="panel-action" id="stop-job"' + (state.jobStopped?' disabled':"") + '>' + (state.jobStopped?"Preview task stopped":"Stop preview task") + '</button><p class="panel-footnote">Illustrative tasks. No agent is running on your computer.</p>',
  audio: () => '<p class="panel-kicker">Output volume</p><div class="volume-value"><output id="volume-output">' + state.volume + '%</output></div><label class="sr-only" for="volume-slider">Sample volume</label><input class="volume-slider" id="volume-slider" type="range" min="0" max="100" value="' + state.volume + '"><div class="device-row"><span>Speakers</span>' + icon("check") + '</div><div class="device-row"><span>Headphones</span><small>Not connected</small></div><p class="panel-footnote">Preview control · Windows volume is unchanged.</p>',
  battery: () => '<p class="panel-kicker">Battery</p><div class="volume-value">82%</div><div class="detail-row"><span>Power source</span><span>Battery</span></div><div class="detail-row"><span>Energy mode</span><span>Balanced</span></div><p class="panel-footnote">Illustrative power state. Real estimates depend on hardware.</p>',
  network: () => '<p class="panel-kicker">Wi-Fi</p><div class="device-row"><span>Home network</span>' + icon("check") + '</div><div class="detail-row"><span>Status</span><span>Connected</span></div><div class="detail-row"><span>Signal</span><span>Excellent</span></div><p class="panel-footnote">Sample connection. Native network settings remain accessible in the application design.</p>',
  tray: () => '<p class="panel-kicker">System tray</p><div class="detail-row"><span>Cloud sync</span><span>Up to date</span></div><div class="detail-row"><span>Security</span><span>No alerts</span></div><div class="detail-row"><span>Input language</span><span>ENG</span></div><p class="panel-footnote">Sample status. The first release keeps a route to the native Windows tray.</p>',
  calendar: () => '<p class="panel-kicker">Tuesday, October 20</p><div class="calendar-date">October 2026</div><div class="calendar-grid">' + ["M","T","W","T","F","S","S"].map(t=>'<span class="weekday">' + t + '</span>').join("") + '<span></span><span></span><span></span>' + Array.from({length:31},(_,i)=>'<span' + (i===19?' class="today"':"") + '>' + (i+1) + '</span>').join("") + '</div><p class="panel-footnote">Sample calendar · no calendar account connected.</p>',
  overflow: () => '<p class="panel-kicker">Your apps</p><div class="result-list">' + Object.keys(apps).map(id=>row(id,"Application preview")).join("") + '</div><p class="panel-subheading">Widgets & controls</p><div class="result-list">' + [["agents","Agents"],["weather","Weather"],["audio","Sound"],["battery","Battery"],["network","Wi-Fi"],["calendar","Calendar"],["tray","System tray"],["customize","Customize dock"]].map(([id,label])=>'<button class="result-row" data-open="' + id + '">' + icon(id==="weather"?"sun":id==="network"?"wifi":id==="calendar"?"tray":id==="customize"?"settings":id) + '<span><b>' + label + '</b></span>' + icon("arrow") + '</button>').join("") + '</div>',
  customize: () => '<p class="panel-kicker">Make the dock yours.</p><label class="edit-field" for="weather-size">Weather size</label><select class="panel-select" id="weather-size"><option value="compact">Compact · condition & temperature</option><option value="wide">Standard · city & temperature</option><option value="rich">Rich · tomorrow at a glance</option></select><label class="edit-check"><input type="checkbox" id="show-weather">Show weather</label><label class="edit-check"><input type="checkbox" id="join-layout">Join status to the main dock</label><div class="reorder-row"><span>App order</span><button id="reorder-apps">Move first app to end</button></div><p class="panel-footnote">Changes stay in this browser preview. Overflow keeps every item reachable on smaller screens.</p><div class="panel-actions"><button class="panel-action" id="cancel-edit">Cancel</button><button class="panel-action primary" id="save-edit">Save layout</button></div>'
 };
 const titles = {start:"Your workspace",search:"Proton Search",weather:"Weather",agents:"Agents",audio:"Sound",battery:"Power",network:"Network",tray:"System tray",calendar:"Calendar",overflow:"Everything within reach",customize:"Customize dock",...apps};
 function open(id, button) {
  if (opened===id && !panel.hidden) { close(); return; }
  close(false);
  invoker=button; opened=id;
  heading.textContent=titles[id] || "Preview";
  if (id==="customize") {
   editSnapshot={...state}; orderSnapshot=[...appGroup.children];
   stage.classList.add("editing"); document.getElementById("edit-cue").hidden=false;
  }
  content.innerHTML=templates[id] ? templates[id]() : '<div class="app-preview">' + appImage(id) + '<h3>' + apps[id] + '</h3><p>' + (state.profile==="presentation"?"Private content is hidden in this profile.":"Your pinned application, one click away.") + '</p><p class="panel-footnote">Illustrative application icon. This opens a preview only.</p></div>';
  panel.hidden=false;
  if (button && button.hasAttribute("aria-expanded")) button.setAttribute("aria-expanded","true");
  attach(id);
  position();
  (content.querySelector("input[type=search]") || document.getElementById("close-panel")).focus({preventScroll:true});
  announce(titles[id] + " preview opened.");
 }
 function attach(id) {
  if (id==="search") {
   const input=document.getElementById("sample-search"), list=document.getElementById("search-results");
   const items=[{title:"Files",detail:"Application",app:"files"},{title:"Code editor",detail:"Application",app:"code"},{title:"Design brief.pdf",detail:"Document · Projects",app:"files"},{title:"Widget ideas",detail:"Notes · Proton 2.0",app:"files"}];
   function render() {
    list.replaceChildren();
    const matches=items.filter(i=>state.profile!=="presentation" || i.detail==="Application").filter(i=>(i.title+" "+i.detail).toLowerCase().includes(input.value.toLowerCase()));
    if (!matches.length) { const p=document.createElement("p");p.className="condition";p.textContent="No sample matches.";list.appendChild(p); }
    matches.forEach(item=>{
     const row=document.createElement("button");row.className="result-row";row.dataset.open=item.app;
     const img=document.createElement("img");img.src="assets/"+item.app+".svg";img.alt="";
     const text=document.createElement("span"), b=document.createElement("b"), small=document.createElement("small");
     b.textContent=item.title;small.textContent=item.detail;text.append(b,small);row.append(img,text);list.appendChild(row);
    });
   }
   input.addEventListener("input",render);render();
  }
  if (id==="audio") document.getElementById("volume-slider").addEventListener("input",e=>{
   state.volume=Number(e.target.value);document.getElementById("volume-output").textContent=state.volume+"%";
   stage.querySelector('[data-panel="audio"]').setAttribute("aria-label","Sound, "+state.volume+" percent");
  });
  if (id==="start") {
   const profile=document.getElementById("panel-profile");profile.value=state.profile;
   profile.addEventListener("change",e=>changeProfile(e.target.value));
  }
  if (id==="agents") document.getElementById("stop-job")?.addEventListener("click",()=>{
   state.jobStopped=true;content.innerHTML=templates.agents();attach("agents");position();document.getElementById("close-panel").focus();announce("Sample task cancelled. No system action performed.");
  });
  if (id==="customize") {
   const size=document.getElementById("weather-size");size.value=state.weatherSize;
   size.addEventListener("change",e=>{state.weatherSize=e.target.value;state.preset="balanced";apply();position();});
   const show=document.getElementById("show-weather");show.checked=state.weatherEnabled;
   show.addEventListener("change",e=>{state.weatherEnabled=e.target.checked;state.preset="balanced";apply();position();});
   const joined=document.getElementById("join-layout");joined.checked=state.joined;
   joined.addEventListener("change",e=>{state.joined=e.target.checked;apply();position();});
   document.getElementById("reorder-apps").addEventListener("click",()=>{appGroup.appendChild(appGroup.firstElementChild);announce("First application moved to end of preview dock.");});
   document.getElementById("cancel-edit").addEventListener("click",()=>{close();announce("Layout changes cancelled.");});
   document.getElementById("save-edit").addEventListener("click",()=>{
    editSnapshot=null;stage.classList.remove("editing");document.getElementById("edit-cue").hidden=true;close();announce("Layout saved for this preview.");
   });
  }
 }
 function changeProfile(value) {
  const returnFocus=panel.contains(document.activeElement) ? stage.querySelector('[data-panel="start"]') : null;
  close(false);state.profile=value;state.preset=value==="presentation"?"essential":"balanced";
  state.weatherSize=value==="home"?"rich":"wide";apply();returnFocus?.focus({preventScroll:true});announce(value+" profile selected.");
 }
 stage.addEventListener("click",e=>{
  const nested=e.target.closest("[data-open]");
  if (nested) {
   // Keep a persistent dock or customization control as the return-focus target.
   const destination=nested.dataset.open;
   const anchor=[...stage.querySelectorAll("[data-panel]")].find(b=>b.dataset.panel===destination && b.getClientRects().length) || invoker || document.getElementById("customize");
   open(destination,anchor);return;
  }
  const button=e.target.closest("[data-panel]");
  if (button) {open(button.dataset.panel,button);return;}
 });
 document.getElementById("close-panel").addEventListener("click",()=>close());
 document.getElementById("customize").addEventListener("click",e=>open("customize",e.currentTarget));
 document.addEventListener("keydown",e=>{if(e.key==="Escape"&&!panel.hidden){e.preventDefault();close();}});
 document.addEventListener("pointerdown",e=>{
  if (!panel.hidden && !panel.contains(e.target) && !e.target.closest("[data-panel]") && !e.target.closest("#customize")) close(false);
 });
 controls.preset.addEventListener("change",e=>{const value=e.target.value;close(false);state.preset=value;state.weatherSize=state.preset==="rich"?"rich":"wide";state.weatherEnabled=true;apply();announce("Composition changed to "+e.target.selectedOptions[0].textContent+".");});
 ["wallpaper","material"].forEach(key=>controls[key].addEventListener("change",e=>{const value=e.target.value;close(false);state[key]=value;apply();}));
 controls.profile.addEventListener("change",e=>changeProfile(e.target.value));
 new ResizeObserver(position).observe(stage);
 apply();
})();
