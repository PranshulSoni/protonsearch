"use strict";
(() => {
 const dock = document.getElementById("dock");
 if (!dock) return;
 const flyout = document.getElementById("flyout");
 const title = document.getElementById("flyout-title");
 const body = document.getElementById("flyout-body");
 const status = document.getElementById("demo-status");
 const tiles = [...dock.querySelectorAll("button[data-panel]")];
 let trigger = null;
 const panels = {
 start: ["Your Start", '<p class="sample-tag">SAMPLE APP DRAWER</p><div class="sample-row"><span>Code</span><small>Pinned</small></div><div class="sample-row"><span>Browser</span><small>Pinned</small></div><div class="sample-row"><span>File Explorer</span><small>All apps</small></div><p>Custom layout, cached app inventory, keyboard-first navigation.</p>'],
 search: ["Proton Search", '<label for="sample-search">Search the sample catalog</label><input type="search" id="sample-search" placeholder="Try files, code, notes…" autocomplete="off"><div id="search-results"></div><p>Preview only. Search never starts an agent automatically.</p>'],
 agents: ["Agent workspace", '<p class="sample-tag">SAMPLE JOBS · NO EXECUTION</p><div class="sample-row"><span>Research a topic</span><small>Running · local provider</small></div><div class="sample-row"><span>Organize downloads</span><small>Awaiting approval</small></div><p>Scoped permissions, visible tools, independent jobs and a stop control.</p>'],
 apps: ["Applications", '<div class="sample-row"><span>Code</span><small>2 windows</small></div><div class="sample-row"><span>Browser</span><small>1 window</small></div><p>Pinned apps and running windows share identity, not just an executable filename.</p>'],
 files: ["Pinned workspace", '<div class="sample-row"><span>Proton 2.0 design</span><small>Document</small></div><div class="sample-row"><span>Projects</span><small>Folder</small></div><p>Choose folders and preview recent items; privacy redaction follows the profile.</p>'],
 weather: ["Pune weather", '<p class="sample-tag">ILLUSTRATIVE FORECAST</p><div class="sample-row"><span>Today · Clear</span><b>26°</b></div><div class="sample-row"><span>Tomorrow · Partly cloudy</span><b>28° / 21°</b></div><div class="sample-row"><span>Thursday · Light rain</span><b>25° / 20°</b></div><p>Manual city selection, cached data and a visible refresh age in the real widget.</p>'],
 audio: ["Sound", '<label for="sample-volume">Sample volume: <output id="sample-volume-value">64%</output></label><input id="sample-volume" type="range" min="0" max="100" value="64"><p>Speakers · Default device</p><small>This changes only the preview, not Windows volume.</small>'],
 battery: ["Power", '<div class="sample-row"><span>Battery</span><b>82%</b></div><p>About 5 hours remaining · sample data</p><p>The real widget responds to hardware and OS power events.</p>'],
 tray: ["Native tray access", '<p>The first tray widget opens the native Windows surface. This preview does not control Windows.</p><p>Rehosting other apps’ tray icons needs separate compatibility research.</p>'],
 clock: ["Calendar", '<p class="sample-tag">TUESDAY · 20 OCTOBER 2026</p><div class="sample-row"><span>09:41</span><small>Sample local time</small></div><p>A compact clock can expand into a calendar. Connected agendas are a later opt-in.</p>']
 };
 function close(restore = false) {
  flyout.hidden = true;
  status.textContent = "Preview closed.";
  tiles.forEach(t => t.setAttribute("aria-expanded", "false"));
  if (restore && trigger) trigger.focus();
  trigger = null;
 }
 function open(tile) {
  if (trigger === tile && !flyout.hidden) { close(true); return; }
  close();
  trigger = tile;
  const [heading, html] = panels[tile.dataset.panel];
  title.textContent = heading;
  body.innerHTML = html;
  flyout.hidden = false;
  tile.setAttribute("aria-expanded", "true");
  document.getElementById("close-flyout").focus();
  status.textContent = heading + " preview opened.";
  if (tile.dataset.panel === "search") {
   const input = document.getElementById("sample-search");
   const samples = ["Files · Design brief.pdf", "Code · protonsearch", "Notes · Widget ideas", "App · File Explorer"];
   const render = () => {
    const results = document.getElementById("search-results");
    const matches = samples.filter(s => s.toLowerCase().includes(input.value.toLowerCase()));
    results.replaceChildren();
    (matches.length ? matches : ["No sample matches"]).forEach(s => {
     const row = document.createElement("div"); row.className = "sample-row"; row.textContent = s; results.appendChild(row);
    });
   };
   input.addEventListener("input", render); render(); input.focus();
  }
  if (tile.dataset.panel === "audio") {
   document.getElementById("sample-volume").addEventListener("input", event => {
    const value = event.target.value + "%";
    document.getElementById("sample-volume-value").textContent = value;
    tile.querySelector("small").textContent = value;
   });
  }
 }
 tiles.forEach(tile => {
  tile.setAttribute("aria-expanded", "false");
  tile.setAttribute("aria-controls", "flyout");
  tile.addEventListener("click", () => open(tile));
 });
 document.getElementById("close-flyout").addEventListener("click", () => close(true));
 document.addEventListener("keydown", event => {
  if (event.key === "Escape" && !flyout.hidden) { event.preventDefault(); close(true); status.textContent = "Preview closed."; }
 });
 document.addEventListener("click", event => {
  if (!flyout.hidden && !flyout.contains(event.target) && !dock.contains(event.target)) close();
 });
 document.getElementById("widget-size").addEventListener("change", event => {
  document.getElementById("weather-tile").className = "tile weather " + event.target.value;
  status.textContent = "Weather size: " + event.target.selectedOptions[0].textContent;
 });
 document.getElementById("edit-layout").addEventListener("click", event => {
  close();
  const editing = dock.classList.toggle("editing");
  event.currentTarget.setAttribute("aria-pressed", String(editing));
  status.textContent = editing ? "Edit affordance preview. Reordering and saving belong to implementation." : "Edit preview closed.";
 });
 document.getElementById("profile").addEventListener("change", event => {
  close(); const profile = event.target.value;
  document.getElementById("workspace-label").textContent = profile.toUpperCase() + " PROFILE";
  document.getElementById("workspace-caption").textContent = {
   Work: "Tools, context, and actions in one quiet place.",
   Home: "Your files, forecast and a little room to unwind.",
   Presentation: "Fewer distractions. Private details tucked away."
  }[profile];
  tiles.forEach(tile => {
   tile.hidden = (profile === "Presentation" && ["agents", "files", "weather"].includes(tile.dataset.panel)) ||
                 (profile === "Home" && tile.dataset.panel === "agents");
  });
  status.textContent = profile + " sample composition active. No tasks were started or stopped.";
 });
})();
