"use strict";
(() => {
  const library=window.ProtonWidgets,stage=document.querySelector('.dock-stage');
  if(!library||!stage)return;
  const {icon,escape:esc}=library;
  const clamp=(value,min,max)=>Math.min(max,Math.max(min,value));
  const bounded=(value,min,max,fallback=min)=>Number.isFinite(Number(value))?clamp(Number(value),min,max):fallback;
  const format=seconds=>`${String(Math.floor(seconds/60)).padStart(2,'0')}:${String(Math.floor(seconds%60)).padStart(2,'0')}`;
  const apps=[{id:'notes',name:'Notes',letter:'N',detail:'Capture ideas, keep the good ones',content:'A quieter workspace\n\nOne place for your apps, your focus and the little things that keep the day moving.'},{id:'browser',name:'Browser',letter:'B',detail:'Your next discovery starts here',content:'Reading list\n\nThe art of noticing\nDesigning for calm\nSmall tools, meaningful work'},{id:'code',name:'Code',letter:'C',detail:'Build something that matters',content:'proton / workspace\n\nREADME.md\nsrc / main.rs\ndocs / design.md'},{id:'calendar',name:'Calendar',letter:'D',detail:'Make space in your day',content:'Today\n\n10:00  Design review\n14:30  Deep work\n17:00  Walk outside'},{id:'files',name:'Files',letter:'F',detail:'Everything, in its place',content:'Recent files\n\nProduct brief.pdf\nAutumn playlist.txt\nWeekly notes.md'},{id:'studio',name:'Studio',letter:'S',detail:'A canvas for your next idea',content:'Untitled canvas\n\nStart with a line. Give the idea some room.'}];
  const files=[{name:'Product brief.pdf',type:'Document',icon:'file',detail:'Updated today · 248 KB',body:'Proton is a calmer way to move through your day. This sample brief brings search, useful widgets and a focused workspace together.'},{name:'Weekly notes.md',type:'Document',icon:'file',detail:'Updated 2 hours ago · 4 KB',body:'This week: simplify the dock, finish the interaction study, and make room for uninterrupted work.'},{name:'Design system',type:'Folder',icon:'apps',detail:'12 sample files',body:'Foundations, typography, spacing, glass surfaces and original icons. A small visual language that scales.'},...apps.map(app=>({name:app.name,type:'Application',icon:'apps',detail:app.detail,body:app.content,app:app.id}))];
  const tracks=[{title:'Weightless',artist:'Marconi Union',album:'Ambient Transmissions',duration:389},{title:'A Walk',artist:'Tycho',album:'Dive',duration:316},{title:'First Breath After Coma',artist:'Explosions in the Sky',album:'The Earth Is Not a Cold Dead Place',duration:573}];
  const state={music:{index:0,playing:false,position:66,updated:Date.now()},focus:{seconds:1500,running:false,deadline:0},energy:{seconds:1800,running:false,deadline:0},volume:68,muted:false,output:'Studio speakers',network:'Proton Studio',wifi:true,running:new Set(['notes']),weatherUnit:'C',weatherTab:'hours',search:'',appFilter:'',result:null,agent:{task:'Summarize the product brief',status:'idle',started:0,step:0}};
  const detail=document.createElement('section');
  detail.id='widgetDetail';detail.className='widget-detail';detail.hidden=true;
  detail.setAttribute('role','dialog');detail.setAttribute('aria-labelledby','widgetDetailTitle');
  detail.innerHTML=`<div class="detail-head detail-header"><div class="detail-heading"><span class="detail-eyebrow">PROTON / WIDGET</span><h2 id="widgetDetailTitle"></h2></div><button class="widget-action detail-icon-button detail-close" type="button" data-detail-action="close" aria-label="Close widget details">${icon('close')}</button></div><div class="detail-body"></div><div class="detail-footer">Interactive study · local sample data</div>`;
  document.body.append(detail);
  const body=detail.querySelector('.detail-body'),heading=detail.querySelector('h2');
  let key=null,anchor=null,trigger=null,positionFrame=null;
  const timers=new Map();
  const button=(label,action,{primary=false,pressed=null,extra=''}={})=>`<button type="button" class="detail-button${primary?' detail-primary':''}" data-detail-action="${action}"${pressed===null?'':` aria-pressed="${pressed}"`} ${extra}>${label}</button>`;
  const iconButton=(label,action,glyph)=>`<button type="button" class="widget-action detail-icon-button" data-detail-action="${action}" aria-label="${label}">${icon(glyph)}</button>`;
  const notice=text=>`<p class="detail-notice">${esc(text)}</p>`;
  const row=(title,subtitle,glyph='file',action='',meta='')=>`<${action?'button type="button"':'div'} class="detail-row" ${action?`data-detail-action="${action}"`:''}><span class="detail-row-icon">${icon(glyph)}</span><span class="detail-row-copy"><b>${esc(title)}</b><small>${esc(subtitle)}</small></span>${meta?`<span class="detail-row-meta">${esc(meta)}</span>`:''}</${action?'button':'div'}>`;
  const hero=(value,label)=>`<div class="detail-hero"><strong class="detail-hero-value">${esc(value)}</strong><span class="detail-muted">${esc(label)}</span></div>`;
  function positionPanel(){
    positionFrame=null;
    if(!anchor||detail.hidden)return;
    const rect=anchor.getBoundingClientRect(),width=Math.min(380,innerWidth-24);
    const profileTop=stage.querySelector('.mode-switch')?.getBoundingClientRect().top??rect.top;
    const baseline=Math.min(rect.top,profileTop)-12;
    detail.style.width=`${width}px`;detail.style.maxHeight=`${Math.max(1,Math.min(innerHeight-24,baseline-12))}px`;
    const height=detail.getBoundingClientRect().height;
    detail.style.left=`${clamp(rect.left+rect.width/2-width/2,12,Math.max(12,innerWidth-width-12))}px`;
    detail.style.top=`${clamp(baseline-height,12,Math.max(12,innerHeight-height-12))}px`;
  }
  function queuePosition(){if(positionFrame===null)positionFrame=requestAnimationFrame(positionPanel);}
  function closePanel(restore=true){
    const prior=trigger;
    detail.hidden=true;trigger?.setAttribute('aria-expanded','false');if(anchor)delete anchor.dataset.panelOpen;key=null;anchor=null;trigger=null;
    if(restore&&prior?.isConnected&&!prior.closest('[inert]')&&!prior.disabled)prior.focus({preventScroll:true});
  }
  function openPanel(next,source){
    if(stage.classList.contains('editing'))return;
    if(key===next&&trigger===source){closePanel();return;}
    trigger?.setAttribute('aria-expanded','false');if(anchor)delete anchor.dataset.panelOpen;
    key=next;trigger=source;anchor=source.closest('.widget')||source;
    source.setAttribute('aria-haspopup','dialog');source.setAttribute('aria-expanded','true');anchor.dataset.panelOpen='true';detail.hidden=false;renderPanel();
    const first=body.querySelector('input:not([type=range]),textarea')||detail.querySelector('.detail-close');
    first?.focus({preventScroll:true});
  }
  function renderPanel(preserve=true){
    if(!key)return;
    const focus=preserve&&detail.contains(document.activeElement)?document.activeElement:null;
    const focusId=focus?.id,focusAction=focus?.dataset.detailAction,selection=focus&&typeof focus.selectionStart==='number'?[focus.selectionStart,focus.selectionEnd]:null;
    const scroll=body.scrollTop;
    let title=library.byId.get(key)?.name||'Widget',html='';
    if(key==='search')html=renderSearch();
    else if(key==='apps'||key==='launcher')html=renderApps();
    else if(key.startsWith('app:')){const app=apps.find(item=>item.id===key.slice(4));title=app?.name||'Application';html=renderApp(app);}
    else if(key==='weather')html=renderWeather();
    else if(key==='music')html=renderMusic();
    else if(key==='quick-controls'||key==='network')html=renderControls();
    else if(key==='focus'||key==='energy')html=renderFocus(key);
    else if(key==='agents'||key==='chatgpt')html=renderAgent();
    else if(key==='time'||key==='analog'){title='Your calendar';html=renderCalendar();}
    else if(key.startsWith('custom-')){title=anchor?.dataset.widgetName||'Custom metric';html=renderMetric();}
    else html=renderExtra(key);
    heading.textContent=title;body.innerHTML=html;body.scrollTop=scroll;
    if(focusId){const replacement=document.getElementById(focusId);replacement?.focus({preventScroll:true});if(selection&&replacement?.setSelectionRange)replacement.setSelectionRange(...selection);}
    else if(focusAction){const replacement=[...body.querySelectorAll('[data-detail-action]')].find(node=>node.dataset.detailAction===focusAction);(replacement||detail.querySelector('.detail-close')).focus({preventScroll:true});}
    if(focus&&!detail.contains(document.activeElement)){const fallback=body.querySelector('button:not([disabled]),input:not([disabled]),textarea:not([disabled]),select:not([disabled])')||detail.querySelector('.detail-close');fallback.focus({preventScroll:true});}
    positionPanel();
  }
  function renderSearch(){
    const query=state.search.toLowerCase(),results=files.filter(file=>`${file.name} ${file.type} ${file.detail}`.toLowerCase().includes(query));
    const result=state.result===null?null:files[state.result];
    return `<label class="detail-search" for="widgetSearch">${icon('search')}<input id="widgetSearch" aria-label="Search sample files and apps" class="detail-input" data-detail-input="search" placeholder="Search sample files and apps" value="${esc(state.search)}" autocomplete="off"/></label><p class="detail-section-title">${query?`${results.length} results`:'Suggested for you'}</p><div class="detail-list">${results.length?results.map(file=>row(file.name,file.detail,file.icon,`result:${files.indexOf(file)}`,file.type)).join(''):notice('No matches. Try “notes”, “design” or “code”.')}</div>${result?`<section class="app-preview"><div class="app-preview-bar">${icon(result.icon)}<b>${esc(result.name)}</b></div><div class="app-preview-content">${esc(result.body)}</div>${result.app?button('Open app preview',`app:${result.app}`,{primary:true}):''}</section>`:''}`;
  }
  function renderApps(){
    const list=apps.filter(app=>app.name.toLowerCase().includes(state.appFilter.toLowerCase()));
    return `<label class="detail-field" for="appSearch">Find an application<input id="appSearch" class="detail-input" data-detail-input="apps" placeholder="Filter apps" value="${esc(state.appFilter)}" autocomplete="off"/></label><div class="detail-list">${list.length?list.map(app=>row(app.name,app.detail,'apps',`app:${app.id}`,state.running.has(app.id)?'Running':'Open')).join(''):notice('No apps match this filter.')}</div>${notice('Applications open as sample window previews in this study.')}`;
  }
  function renderApp(app){
    if(!app)return notice('This sample application is unavailable.');
    return `<div class="app-preview"><div class="app-preview-bar"><span class="app-monogram app-${app.id}">${app.letter}</span><b>${esc(app.name)}</b><span class="detail-row-meta">Running</span></div><div class="app-preview-content">${esc(app.content)}</div></div><div class="detail-actions">${button('All apps','all-apps')}${button('Close application',`quit:${app.id}`)}</div>${notice('Sample window preview. No application was launched on your computer.')}`;
  }
  function temperature(value){return `${state.weatherUnit==='C'?value:Math.round(value*9/5+32)}°`;}
  function renderWeather(){
    const hours=[['Now',31,'sun'],['17:00',29,'weather'],['18:00',28,'weather'],['19:00',27,'weather'],['20:00',26,'weather'],['21:00',26,'weather']];
    const days=[['Today',31,'sun'],['Wed',32,'sun'],['Thu',30,'weather'],['Fri',29,'weather'],['Sat',30,'sun'],['Sun',31,'sun'],['Mon',32,'sun']];
    const forecast=state.weatherTab==='hours'?hours:days;
    return `<div class="detail-actions"><span class="detail-muted">Ahmedabad, India</span><div class="detail-tabs">${button('°C','weather-C',{pressed:state.weatherUnit==='C'})}${button('°F','weather-F',{pressed:state.weatherUnit==='F'})}</div></div>${hero(temperature(31),'Mostly sunny · feels like '+temperature(33))}<div class="detail-tabs">${button('Hourly','weather-hours',{pressed:state.weatherTab==='hours'})}${button('7-day outlook','weather-days',{pressed:state.weatherTab==='days'})}</div><div class="forecast-grid">${forecast.map(([label,temp,glyph])=>`<div class="forecast-item"><span>${label}</span>${icon(glyph)}<b>${temperature(temp)}</b></div>`).join('')}</div><div class="detail-grid"><div class="detail-stat"><small>Humidity</small><b>48%</b></div><div class="detail-stat"><small>Wind</small><b>12 km/h</b></div><div class="detail-stat"><small>UV index</small><b>4 · moderate</b></div></div>${notice('Illustrative forecast. Weather data is not live.')}`;
  }
  function renderMusic(){
    const track=tracks[state.music.index];
    return `<div class="music-hero"><div class="music-art music-cover"><span></span></div><div class="music-detail-copy"><h3>${esc(track.title)}</h3><p class="detail-muted">${esc(track.artist)} · ${esc(track.album)}</p></div></div><label class="detail-field" for="musicSeek">Playback position<input id="musicSeek" class="detail-range" type="range" min="0" max="${track.duration}" value="${Math.floor(state.music.position)}" data-detail-input="seek" aria-valuetext="${format(state.music.position)} of ${format(track.duration)}"/></label><div class="detail-actions"><span data-music-position>${format(state.music.position)}</span><span>${format(track.duration)}</span></div><div class="music-controls playback-controls">${iconButton('Previous track','music-previous','previous')}${iconButton(state.music.playing?'Pause music':'Play music','music-play',state.music.playing?'pause':'play')}${iconButton('Next track','music-next','next')}</div>${notice('Playback is a visual simulation. No audio is streamed.')}`;
  }
  function renderControls(){
    return `<div class="detail-actions"><b>Volume</b>${button(state.muted?'Unmute':'Mute','mute',{pressed:state.muted})}</div><label class="detail-field" for="volumeRange">${state.muted?'Muted':`${state.volume}% volume`}<input class="detail-range" id="volumeRange" type="range" min="0" max="100" value="${state.volume}" data-detail-input="volume"/></label><label class="detail-field" for="audioOutput">Audio output<select id="audioOutput" class="detail-input" data-detail-input="output">${['Studio speakers','Headphones','Monitor audio'].map(name=>`<option${state.output===name?' selected':''}>${name}</option>`).join('')}</select></label><div class="detail-actions"><h3 class="detail-section-title">Wi-Fi</h3>${button(state.wifi?'On':'Off','wifi',{pressed:state.wifi})}</div><div class="detail-list">${['Proton Studio','Guest network','Mobile hotspot'].map(name=>row(name,state.wifi?(state.network===name?'Connected · secured':'Available · secured'):'Wi-Fi is off','wifi',`network:${name}`,state.wifi&&state.network===name?'Connected':'')).join('')}</div>${notice('These controls change the demo only; system settings stay on your device.')}`;
  }
  function renderFocus(which){
    const timer=state[which],isFocus=which==='focus';
    return `${hero(format(timer.seconds),timer.running?(isFocus?'One thing at a time. You have this.':'Step away, stretch, breathe.'):(timer.seconds===0?'Session complete. Well done.':isFocus?'Protect a little time for meaningful work.':'Give yourself a little breathing room.'))}<div class="detail-actions">${button(timer.running?'Pause':timer.seconds===0?'Start again':'Start',`${which}-play`,{primary:true})}${button('Reset',`${which}-reset`)}</div><div class="detail-progress" role="progressbar" aria-label="Session progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${Math.round((1-timer.seconds/(isFocus?1500:1800))*100)}"><span style="width:${(1-timer.seconds/(isFocus?1500:1800))*100}%"></span></div>${notice(isFocus?'25-minute focus session. The timer continues while this panel is closed.':'30-minute recharge session. Pause or reset whenever you need.')}`;
  }
  function renderAgent(){
    const agent=state.agent,labels=['Read the sample brief','Find the key ideas','Draft a concise summary'];
    const busy=agent.status==='running';
    return `${notice('Demo agent · runs only on the sample content in this page.')}<form id="agentForm"><label class="detail-field" for="agentTask">What would you like to work on?<textarea id="agentTask" class="detail-input" rows="3" maxlength="180" ${busy?'disabled':''}>${esc(agent.task)}</textarea></label><div class="detail-actions">${busy?button('Cancel run','agent-cancel'):`<button class="detail-button detail-primary" type="submit">${agent.status==='complete'?'Run again':'Run sample task'}</button>`}<span class="detail-muted" role="status">${busy?'Working on sample content':agent.status==='cancelled'?'Run cancelled':agent.status==='complete'?'Completed':'Ready when you are'}</span></div></form>${agent.status==='idle'?'':`<div class="detail-list">${labels.map((label,i)=>`<div class="agent-stage ${agent.status==='complete'||i<agent.step?'complete':busy&&i===agent.step?'active':''}"><span>${icon(agent.status==='complete'||i<agent.step?'check':'agents')}</span>${label}</div>`).join('')}</div>`}${agent.status==='complete'?`<section class="app-preview"><div class="app-preview-bar"><b>Sample result</b></div><div class="app-preview-content"><strong>Task: ${esc(agent.task)}</strong><p>Proton brings everyday tools into one calm workspace. Search keeps files close, focused sessions protect attention, and small adaptive widgets make useful information easy to reach.</p><p>Next step: test the narrow layout and refine the interactions that matter most.</p></div></section>`:''}`;
  }
  function renderCalendar(){
    const now=new Date(),first=new Date(now.getFullYear(),now.getMonth(),1),days=new Date(now.getFullYear(),now.getMonth()+1,0).getDate();
    return `${hero(now.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'}),now.toLocaleDateString([],{weekday:'long',month:'long',day:'numeric'}))}<h3 class="detail-section-title">${now.toLocaleDateString([],{month:'long',year:'numeric'})}</h3><div class="calendar-grid">${['S','M','T','W','T','F','S'].map(day=>`<span class="calendar-weekday">${day}</span>`).join('')}${'<span aria-hidden="true"></span>'.repeat(first.getDay())}${Array.from({length:days},(_,i)=>`<span class="calendar-day${i+1===now.getDate()?' today':''}"${i+1===now.getDate()?' aria-current="date"':''}>${i+1}</span>`).join('')}</div><p class="detail-section-title">A little space for today</p>${row('Design review','10:00 – 10:30 · Sample event','clock')}${row('Deep work','14:30 – 16:00 · Sample event','focus')}`;
  }
  function renderMetric(){
    const label=anchor?.querySelector('.custom-label')?.textContent||'',value=anchor?.querySelector('.custom-value')?.textContent||'',info=anchor?.querySelector('.custom-detail')?.textContent||'';
    return `<form id="metricEditForm"><label class="detail-field" for="metricEditLabel">Label<input id="metricEditLabel" class="detail-input" maxlength="40" required value="${esc(label)}"/></label><label class="detail-field" for="metricEditValue">Value<input id="metricEditValue" class="detail-input" maxlength="32" required value="${esc(value)}"/></label><label class="detail-field" for="metricEditInfo">Detail<input id="metricEditInfo" class="detail-input" maxlength="80" value="${esc(info)}"/></label><button type="submit" class="detail-button detail-primary">Save metric</button><p id="metricEditStatus" role="status" class="detail-muted">Plain text, saved in this browser.</p></form>`;
  }
  function renderExtra(id){
    if(id==='downloads')return `${hero('3 files','Your latest sample downloads')}<div class="detail-list">${row('Product brief.pdf','248 KB · Complete','file','download:0','Ready')}${row('Dock assets.zip','4.2 MB · Complete','download','download:1','Ready')}${row('Weekly notes.md','4 KB · Complete','file','download:2','Ready')}</div><div id="extraResult" role="status"></div>`;
    if(id==='gmail')return `<div class="detail-list">${row('Maya Chen','Design review · The latest dock study looks great. Let’s review the details at 10.','mail','mail:0','09:12')}${row('Proton team','Weekly notes · Three small improvements for a calmer workspace.','mail','mail:1','Yesterday')}</div><div id="extraResult" role="status"></div>${notice('Sample inbox. No mail account is connected.')}`;
    if(id==='dropbox')return `${hero('All caught up','Your sample workspace is in sync')}<div class="detail-list">${row('Design system','12 files · Synced just now','cloud','cloud:0')}${row('Product brief.pdf','248 KB · Available offline','file','cloud:1')}</div><div id="extraResult" role="status"></div>`;
    const metrics={activity:['79%','Daily movement goal',[['Steps','6,320'],['Distance','4.8 km'],['Active time','42 min']]],followers:['22,400','People following your work',[['This month','+482'],['Engagement','4.8%'],['Top post','1.2K views']]],revenue:['$5,284','Net revenue · last 30 days',[['Previous period','$4,684'],['Growth','+12.8%'],['Subscriptions','94']]],sales:['128','Orders · this month',[['Gross sales','$6,140'],['Average order','$47.97'],['Returns','2']]],visitors:['16,001','Visitors · last 30 days',[['Growth','+8.2%'],['Page views','32,480'],['Avg. visit','2m 14s']]],disk:['71.8 GB','9% of 800 GB used',[['Documents','21.4 GB'],['Applications','32.1 GB'],['Other','18.3 GB']]]};
    const metric=metrics[id];
    return metric?`${hero(metric[0],metric[1])}<svg class="widget-spark detail-chart" viewBox="0 0 300 72" fill="none" aria-label="Sample trend increasing over the last month" role="img"><path d="M0 60 22 54 44 59 66 42 88 46 110 32 132 39 154 22 176 29 198 18 220 22 242 10 264 16 300 4" stroke="currentColor" stroke-width="2"/></svg><div class="detail-grid">${metric[2].map(([label,value])=>`<div class="detail-stat"><small>${label}</small><b>${value}</b></div>`).join('')}</div>${notice('Illustrative analytics. No external service is connected.')}`:notice('This widget uses local sample data.');
  }
  function live(name,text){document.querySelectorAll(`[data-live="${name}"]`).forEach(node=>{node.textContent=text;});}
  function updateTiles(){
    const track=tracks[state.music.index];live('music',track.title);live('artist',track.artist);live('focus',format(state.focus.seconds));live('energy',format(state.energy.seconds));live('weather',temperature(31));
    live('agents',state.agent.status==='running'?'Working…':state.agent.status==='complete'?'Summary ready':'Ready to help');
    live('network',state.wifi?state.network:'Wi-Fi off');
    document.querySelectorAll('.widget-weather .weather-hour').forEach((node,index)=>{node.lastChild.textContent=temperature(index?29:31);});
    document.querySelectorAll('[data-widget-action="music-play"],[data-widget-action="focus-play"],[data-widget-action="energy-play"]').forEach(node=>{
      const type=node.dataset.widgetAction.split('-')[0],running=type==='music'?state.music.playing:state[type].running;
      node.innerHTML=icon(running?'pause':'play');node.setAttribute('aria-label',`${running?'Pause':'Start'} ${type==='music'?'music':type+' timer'}`);node.setAttribute('aria-pressed',String(running));
    });
    document.querySelectorAll('[data-running]').forEach(node=>{node.hidden=!state.running.has(node.dataset.running);});
    document.querySelectorAll('[data-widget-action="controls"]').forEach(node=>{node.innerHTML=icon(state.muted?'mute':'volume');});
  }
  function updateClock(){
    const now=new Date();live('time',now.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit',hour12:false}));
    document.querySelectorAll('.widget-time .widget-secondary').forEach(node=>{node.textContent=now.toLocaleDateString([],{weekday:'short',month:'short',day:'numeric'});});
    live('analog',now.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'}));
    if(key==='time'||key==='analog'){const value=body.querySelector('.detail-hero-value');if(value)value.textContent=now.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'});}
    const menu=document.querySelector('#menuClock');if(menu)menu.textContent=now.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'});
  }
  function refreshTimer(which){
    const timer=state[which];if(!timer.running)return;
    timer.seconds=Math.max(0,Math.ceil((timer.deadline-Date.now())/1000));
    if(timer.seconds===0){timer.running=false;clearInterval(timers.get(which));timers.delete(which);}
    live(which,format(timer.seconds));
    if(key===which){const heroNode=body.querySelector('.detail-hero-value');if(heroNode)heroNode.textContent=format(timer.seconds);const progress=body.querySelector('[role=progressbar]');if(progress){const percent=(1-timer.seconds/(which==='focus'?1500:1800))*100;progress.setAttribute('aria-valuenow',String(Math.round(percent)));progress.firstElementChild.style.width=`${percent}%`;}if(!timer.running)renderPanel();}
    if(!timer.running)updateTiles();
  }
  function refreshMusic(){
    if(!state.music.playing)return;
    const track=tracks[state.music.index],now=Date.now();state.music.position+=(now-state.music.updated)/1000;state.music.updated=now;
    if(state.music.position>=track.duration){state.music.index=(state.music.index+1)%tracks.length;state.music.position=0;updateTiles();if(key==='music')renderPanel();}
    if(key==='music'){const seek=body.querySelector('#musicSeek'),time=body.querySelector('[data-music-position]');if(seek&&document.activeElement!==seek)seek.value=String(Math.floor(state.music.position));if(seek)seek.setAttribute('aria-valuetext',`${format(state.music.position)} of ${format(tracks[state.music.index].duration)}`);if(time)time.textContent=format(state.music.position);}
  }
  function refreshAgent(){
    const agent=state.agent;if(agent.status!=='running')return;
    const next=Math.floor((Date.now()-agent.started)/1100);
    if(next===agent.step)return;agent.step=Math.min(3,next);
    if(next>=3){agent.status='complete';clearInterval(timers.get('agent'));timers.delete('agent');}
    updateTiles();if(key==='agents'||key==='chatgpt')renderPanel();
  }
  function clearTimers(){timers.forEach(handle=>clearInterval(handle));timers.clear();}
  function syncTimers(){
    clearTimers();if(document.hidden)return;
    updateClock();timers.set('clock',setInterval(updateClock,1000));
    for(const which of ['focus','energy'])if(state[which].running){refreshTimer(which);if(state[which].running)timers.set(which,setInterval(()=>refreshTimer(which),1000));}
    if(state.music.playing){refreshMusic();timers.set('music',setInterval(refreshMusic,1000));}
    if(state.agent.status==='running'){refreshAgent();if(state.agent.status==='running')timers.set('agent',setInterval(refreshAgent,300));}
  }
  function act(action){
    if(action==='close'){closePanel();return;}
    if(action.startsWith('result:')){state.result=bounded(action.slice(7),0,files.length-1);renderPanel(true);return;}
    if(action.startsWith('app:')){const app=apps.find(item=>item.id===action.slice(4));if(!app)return;state.running.add(app.id);key=`app:${app.id}`;updateTiles();renderPanel();return;}
    if(action.startsWith('quit:')){state.running.delete(action.slice(5));key='apps';updateTiles();renderPanel();return;}
    if(action==='all-apps'){key='apps';renderPanel();return;}
    if(action.startsWith('weather-')){const value=action.slice(8);if(['C','F'].includes(value))state.weatherUnit=value;else if(['hours','days'].includes(value))state.weatherTab=value;updateTiles();renderPanel();return;}
    if(action.startsWith('music-')){
      refreshMusic();const music=state.music;
      if(action==='music-play'){music.playing=!music.playing;music.updated=Date.now();}
      else if(action==='music-next'||action==='music-previous'){music.index=(music.index+(action==='music-next'?1:tracks.length-1))%tracks.length;music.position=0;music.updated=Date.now();}
      updateTiles();if(key==='music')renderPanel();syncTimers();return;
    }
    if(/^(focus|energy)-(play|reset)$/.test(action)){
      const [which,operation]=action.split('-'),timer=state[which];refreshTimer(which);
      if(operation==='reset'){timer.running=false;timer.seconds=which==='focus'?1500:1800;}
      else {if(!timer.seconds)timer.seconds=which==='focus'?1500:1800;timer.running=!timer.running;timer.deadline=Date.now()+timer.seconds*1000;}
      updateTiles();if(key===which)renderPanel();syncTimers();return;
    }
    if(action==='mute'){state.muted=!state.muted;updateTiles();renderPanel();return;}
    if(action==='wifi'){state.wifi=!state.wifi;updateTiles();renderPanel();return;}
    if(action.startsWith('network:')){state.network=action.slice(8);state.wifi=true;updateTiles();renderPanel();return;}
    if(action==='agent-cancel'){state.agent.status='cancelled';syncTimers();updateTiles();renderPanel();return;}
    const result=body.querySelector('#extraResult');
    if(result&&action.startsWith('download:'))result.textContent=['Product brief.pdf is ready. This sample file describes the Proton workspace concept.','Dock assets.zip contains the sample icon and surface studies.','Weekly notes.md: simplify the dock, finish the interaction study, protect focus time.'][Number(action.slice(9))]||'';
    if(result&&action.startsWith('mail:'))result.textContent=action==='mail:0'?'Maya Chen · Design review\nThe latest dock study looks great. Let’s walk through the details at 10:00 and check the compact layout.':'Proton team · Weekly notes\nThis week: a clearer search flow, useful focus sessions, and a calmer place for everyday tools.';
    if(result&&action.startsWith('cloud:'))result.textContent=action==='cloud:0'?'Design system · 12 sample files, including typography, spacing, glass surfaces and original SVG icons.':'Product brief.pdf · Available offline. A sample overview of Proton’s calm, adaptive workspace.';
    queuePosition();
  }
  document.addEventListener('click',event=>{
    if(stage.classList.contains('editing'))return;
    const target=event.target.closest('button');if(!target)return;
    const direct=target.dataset.widgetAction;
    if(direct){if(direct==='controls'||direct==='network')openPanel(direct==='controls'?'quick-controls':'network',target);else act(direct);return;}
    if(target.dataset.app){state.running.add(target.dataset.app);updateTiles();openPanel(`app:${target.dataset.app}`,target);return;}
    if(target.dataset.openWidget){if(target.dataset.openWidget==='search'){state.result=null;}openPanel(target.dataset.openWidget,target);return;}
    if(target.dataset.detailAction)act(target.dataset.detailAction);
  });
  function outsideDetail(event){
    if(detail.hidden)return;
    const path=event.composedPath();
    // Preserve the original event ancestry when an action replaces the panel contents.
    if(!path.includes(detail)&&!path.some(node=>node instanceof Element&&node.classList.contains('widget'))){
      const destination=path.some(node=>node instanceof Element&&node.matches('button,a[href],input,select,textarea,[tabindex]'));
      closePanel(!destination);
    }
  }
  // After native pointer focus settles, restore the trigger for a background click.
  // A click on another control keeps that control's focus instead.
  document.addEventListener('click',outsideDetail);
  document.addEventListener('keydown',event=>{
    if(detail.hidden)return;
    if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();closePanel();return;}
    if(event.key==='Tab'&&detail.contains(document.activeElement)){
      const nodes=[...detail.querySelectorAll('button:not([disabled]),input:not([disabled]),textarea:not([disabled]),select:not([disabled])')].filter(node=>!node.hidden);
      if(!nodes.length)return;const first=nodes[0],last=nodes[nodes.length-1];
      if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}
    }
  },true);
  body.addEventListener('input',event=>{
    const input=event.target,type=input.dataset.detailInput;
    if(type==='search'){state.search=input.value.slice(0,120);state.result=null;renderPanel(true);}
    if(type==='apps'){state.appFilter=input.value.slice(0,80);renderPanel(true);}
    if(type==='seek'){state.music.position=bounded(input.value,0,tracks[state.music.index].duration);state.music.updated=Date.now();const time=body.querySelector('[data-music-position]');if(time)time.textContent=format(state.music.position);input.setAttribute('aria-valuetext',`${format(state.music.position)} of ${format(tracks[state.music.index].duration)}`);}
    if(type==='volume'){state.volume=bounded(input.value,0,100,68);state.muted=false;input.parentElement.firstChild.textContent=`${state.volume}% volume`;updateTiles();}
  });
  body.addEventListener('change',event=>{if(event.target.dataset.detailInput==='output'){state.output=event.target.value;renderPanel();}});
  body.addEventListener('submit',event=>{
    event.preventDefault();
    if(event.target.id==='agentForm'){
      if(state.agent.status==='running')return;
      const task=body.querySelector('#agentTask').value.trim().slice(0,180)||'Summarize the product brief';
      Object.assign(state.agent,{task,status:'running',started:Date.now(),step:0});updateTiles();renderPanel();syncTimers();
    }
    if(event.target.id==='metricEditForm'){
      const request={id:key,label:body.querySelector('#metricEditLabel').value,value:body.querySelector('#metricEditValue').value,detail:body.querySelector('#metricEditInfo').value,saved:false};
      document.dispatchEvent(new CustomEvent('proton:metric-update',{detail:request}));
      body.querySelector('#metricEditStatus').textContent=request.saved?'Saved. Your metric is updated in both profiles.':'Enter a label and value. Browser storage must be available to save.';
      if(request.saved)heading.textContent=request.label.trim().slice(0,40);
    }
  });
  ['proton:profile-change','proton:editing-change'].forEach(name=>document.addEventListener(name,()=>closePanel(false)));
  document.addEventListener('visibilitychange',()=>{if(document.hidden){clearTimers();closePanel(false);}else{syncTimers();updateTiles();}});
  window.addEventListener('pagehide',()=>{clearTimers();if(positionFrame!==null)cancelAnimationFrame(positionFrame);positionFrame=null;});
  window.addEventListener('pageshow',()=>syncTimers());
  window.addEventListener('resize',queuePosition);document.addEventListener('scroll',queuePosition,true);
  updateTiles();syncTimers();
})();
