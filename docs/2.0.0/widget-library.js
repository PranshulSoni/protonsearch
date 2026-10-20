"use strict";
(() => {
  const paths = {
    search:'<circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 4 4"/>',
    apps:'<rect x="3" y="3" width="7" height="7" rx="2"/><rect x="14" y="3" width="7" height="7" rx="2"/><rect x="3" y="14" width="7" height="7" rx="2"/><rect x="14" y="14" width="7" height="7" rx="2"/>',
    weather:'<path d="M8 18H6a4 4 0 1 1 1-7.9 6 6 0 0 1 11.8 1.4A3.3 3.3 0 1 1 18 18H8Z"/><path d="M4 5 3 4m8 1V3m7 4 2-1"/>',
    sun:'<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>',
    music:'<path d="M9 17V5l11-2v12M9 8l11-2"/><ellipse cx="6" cy="18" rx="3" ry="2.5"/><ellipse cx="17" cy="16" rx="3" ry="2.5"/>',
    play:'<path d="m9 5 11 7-11 7Z"/>', pause:'<path d="M8 5v14M16 5v14"/>', next:'<path d="m5 5 10 7-10 7ZM19 5v14"/>', previous:'<path d="m19 5-10 7 10 7ZM5 5v14"/>',
    volume:'<path d="M4 9h4l5-4v14l-5-4H4Zm13-1a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14"/>',
    mute:'<path d="M4 9h4l5-4v14l-5-4H4Zm13 0 5 6m0-6-5 6"/>',
    wifi:'<path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0m-10 4a5 5 0 0 1 6 0"/><circle cx="12" cy="20" r=".7"/>',
    focus:'<circle cx="12" cy="13" r="8"/><path d="M9 2h6m-3 7v5l3 2M19 5l2 2"/>',
    clock:'<circle cx="12" cy="12" r="9"/><path d="M12 6v6l4 2"/>',
    agents:'<path d="m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5Z"/>',
    activity:'<path d="M2 13h5l3-8 4 14 3-6h5"/>', download:'<path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/>',
    cloud:'<path d="M6 19a5 5 0 1 1 1-9.9 6 6 0 0 1 11.8 1.4A4.3 4.3 0 1 1 18 19Z"/>',
    people:'<circle cx="9" cy="7" r="3"/><path d="M3 21v-3a6 6 0 0 1 12 0v3m1-17a3 3 0 0 1 0 6m2 5a5 5 0 0 1 3 5"/>',
    revenue:'<path d="M12 2v20m5-15H9a3 3 0 0 0 0 6h6a3 3 0 0 1 0 6H6"/>',
    sales:'<path d="M4 8h16l-1 13H5ZM8 8V6a4 4 0 0 1 8 0v2"/>',
    visitors:'<path d="M3 20V10m6 10V5m6 15v-8m6 8V2"/>', disk:'<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M3 14h18m-5 3h1m-5 0h1"/>',
    mail:'<rect x="3" y="5" width="18" height="14" rx="3"/><path d="m4 7 8 6 8-6"/>',
    metric:'<path d="M4 20h16M6 16v-4m6 4V4m6 12V8"/>', close:'<path d="m6 6 12 12M6 18 18 6"/>',
    arrow:'<path d="M5 12h14m-5-5 5 5-5 5"/>', grip:'<path d="M8 5h.01M16 5h.01M8 12h.01M16 12h.01M8 19h.01M16 19h.01"/>',
    reset:'<path d="M4 10a8 8 0 1 1 1 8M4 4v6h6"/>', check:'<path d="m5 12 4 4L19 6"/>', file:'<path d="M14 3H5v18h14V8ZM14 3v6h5M8 13h8m-8 4h5"/>'
  };
  const escape = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  const icon = name => `<svg class="proton-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] || paths.metric}</svg>`;
  const definitions = [
    ['search','Search','search','Find anything','Files, apps & ideas',142,88],
    ['apps','Applications','apps','Your workspace','3 apps ready',144,82],
    ['weather','Weather','weather','31°','Mostly sunny',126,96],
    ['music','Music','music','Weightless','Marconi Union',180,128],
    ['focus','Focus timer','focus','25:00','Make room to think',116,100],
    ['quick-controls','Quick controls','volume','Sound & Wi-Fi','Connected',96,84],
    ['time','Digital clock','clock','16:29','Tuesday, 20 Oct',104,92],
    ['agents','Agents','agents','Ready to help','Local demo',156,106],
    ['launcher','Quick launcher','apps','Open a workspace','Your favorite apps',150,96],
    ['analog','Analog clock','clock','Local time','Asia / Kolkata',110,82],
    ['activity','Activity','activity','79%','Daily movement goal',116,84],
    ['downloads','Downloads','download','3 files','Ready to review',118,84],
    ['network','Network','wifi','Proton Studio','Strong connection',150,96],
    ['dropbox','Cloud files','cloud','Up to date','Last synced just now',150,96],
    ['energy','Energy timer','sun','30:00','Time for a reset',122,100],
    ['followers','Followers','people','22.4K','+482 this month',128,96],
    ['revenue','Revenue','revenue','$5,284','+12.8% this month',132,96],
    ['sales','Sales','sales','128','Orders this month',122,96],
    ['visitors','Visitors','visitors','16,001','+8.2% this month',116,96],
    ['disk','Disk usage','disk','71.8 GB','9% of 800 GB',132,96],
    ['chatgpt','Assistant','agents','Ask anything','Sample conversation',156,96],
    ['gmail','Mail','mail','Design review','2 unread messages',152,96]
  ].map(([id,name,glyph,value,detail,width,min]) => ({id,name,glyph,value,detail,width,min}));
  const byId = new Map(definitions.map(item => [item.id,item]));
  const action = (name,label,glyph) => `<button type="button" class="widget-action" data-widget-action="${name}" aria-label="${label}">${icon(glyph)}</button>`;
  const copy = item => `<span class="widget-copy"><span class="widget-label">${escape(item.id==='weather'?'Ahmedabad':item.name)}</span><strong class="widget-value" data-live="${item.id}">${escape(item.value)}</strong><span class="widget-secondary">${escape(item.detail)}</span></span>`;
  function body(item) {
    const open = `<button type="button" class="widget-face" data-open-widget="${item.id}" aria-label="Open ${escape(item.name)}" aria-haspopup="dialog" aria-expanded="false">`;
    if(item.id==='music') return `<div class="widget-face music-face"><button class="music-cover" type="button" data-open-widget="music" aria-label="Open music player" aria-haspopup="dialog" aria-expanded="false"><span></span></button><button class="widget-copy music-copy" type="button" data-open-widget="music" aria-label="Open music player" aria-haspopup="dialog" aria-expanded="false"><span class="widget-label">Now playing</span><strong class="widget-value" data-live="music">Weightless</strong><span class="widget-secondary" data-live="artist">Marconi Union</span></button>${action('music-play','Play music','play')}</div>`;
    if(item.id==='focus'||item.id==='energy') return `<div class="widget-face"><button type="button" class="widget-copy focus-copy" data-open-widget="${item.id}" aria-label="Open ${item.name}" aria-haspopup="dialog" aria-expanded="false"><span class="widget-label">${item.id==='energy'?'Recharge':'Focus'}</span><strong class="widget-value" data-live="${item.id}">${item.value}</strong></button>${action(`${item.id}-play`,`Start ${item.name}`,'play')}</div>`;
    if(item.id==='quick-controls') return `<div class="widget-face quick-buttons">${action('controls','Open sound controls','volume')}${action('network','Open Wi-Fi controls','wifi')}</div>`;
    if(item.id==='apps') return `<div class="widget-face"><div class="app-grid">${['notes','browser','code'].map((app,i)=>`<button type="button" class="app-shortcut app-${app}" data-app="${app}" aria-haspopup="dialog" aria-expanded="false" aria-label="Open ${['Notes','Browser','Code'][i]} preview"><span class="app-monogram">${['N','B','C'][i]}</span><span class="running-dot" data-running="${app}" hidden></span></button>`).join('')}</div><button type="button" class="widget-action" data-open-widget="apps" aria-label="Open application drawer" aria-haspopup="dialog" aria-expanded="false">${icon('apps')}</button></div>`;
    const extra=item.id==='weather'?`<span class="weather-forecast widget-secondary"><span class="weather-hour">Now${icon('sun')}31°</span><span class="weather-hour">17:00${icon('weather')}29°</span></span>`:'';
    return `${open}<span class="widget-icon">${icon(item.glyph)}</span>${copy(item)}${extra}</button>`;
  }
  function createWidget(item,{hidden=false}={}) {
    const tile=document.createElement('div');
    tile.className=`tile widget widget-${item.id}${['followers','revenue','sales','visitors','disk'].includes(item.id)?' widget-metric':''}`;
    Object.assign(tile.dataset,{widgetId:item.id,widgetName:item.name,defaultW:String(item.width),minW:String(item.min),maxW:'460',startHidden:String(hidden)});
    tile.innerHTML=`<button type="button" class="widget-drag" aria-label="Move ${escape(item.name)}; arrow keys to reorder" title="Drag to move; arrow keys to reorder">${icon('grip')}</button>${body(item)}<span class="widget-edit-label">${escape(item.name)}</span><button type="button" class="widget-resize" aria-label="Resize ${escape(item.name)}; arrow keys change width" title="Drag sideways; arrow keys to resize"></button>`;
    return tile;
  }
  const profiles={daily:['search','apps','weather','music','focus','quick-controls','time'],founder:['search','agents','revenue','visitors','apps','focus','time']};
  document.querySelectorAll('.dock-panel').forEach(panel=>{
    const visible=profiles[panel.dataset.panel]||profiles.daily;
    const ordered=[...visible,...definitions.map(item=>item.id).filter(id=>!visible.includes(id))];
    panel.replaceChildren(...ordered.map(id=>createWidget(byId.get(id),{hidden:!visible.includes(id)})));
    const empty=document.createElement('div');empty.className='empty-profile';empty.textContent='No widgets enabled · choose Customize to add them';panel.append(empty);
  });
  document.querySelectorAll('[data-static-icon]').forEach(node=>{node.innerHTML=icon(node.dataset.staticIcon);});
  window.ProtonWidgets={definitions,byId,icon,escape,createWidget};
})();
