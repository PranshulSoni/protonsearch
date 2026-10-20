"use strict";
(() => {
if(!document.querySelector('.dock-stage'))return;

const stage=document.querySelector('.dock-stage');
const shell=document.querySelector('.dock-shell');
const track=document.querySelector('#dockTrack');
const panels=[...document.querySelectorAll('.dock-panel')];
const modeButtons=[...document.querySelectorAll('.mode-btn[data-mode]')];
const addButton=document.querySelector('#modeAdd');
const popover=document.querySelector('#settingsPopover');
const widgetOptions=document.querySelector('#widgetOptions');
const serviceWidgetOptions=document.querySelector('#serviceWidgetOptions');
const widgetGallery=document.querySelector('#widgetGallery');
const managerProfile=document.querySelector('#managerProfile');
const editLayoutButton=document.querySelector('#editLayoutButton');
const managerClose=document.querySelector('#managerClose');
const resetLayoutButton=document.querySelector('#resetLayout');
const showAllButton=document.querySelector('#showAllWidgets');
const dockHeightRange=document.querySelector('#dockHeightRange');
const dockHeightValue=document.querySelector('#dockHeightValue');
const appearanceValue=document.querySelector('#appearanceValue');
const appearanceButtons=[...document.querySelectorAll('[data-appearance]')];
const layoutToolbar=document.querySelector('#layoutToolbar');
const selectedWidgetName=document.querySelector('#selectedWidgetName');
const selectedWidgetSize=document.querySelector('#selectedWidgetSize');
const doneEditingButton=document.querySelector('#doneEditing');
const manageWidgetsButton=document.querySelector('#manageWidgets');
const reduced=matchMedia('(prefers-reduced-motion: reduce)');
const STORAGE_KEY='protonUnifiedDockLayoutV1';
const CUSTOM_STORAGE_KEY='protonUnifiedDockCustomWidgetsV1';
const MAX_CUSTOM_WIDGETS=12;
const escapeHTML=value=>String(value).replace(/[&<>"']/g,char=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
const DEFAULT_DOCK_HEIGHT=82;
const MIN_DOCK_HEIGHT=76;
const MAX_DOCK_HEIGHT=142;
const DOCK_VERTICAL_INSET=16;
const PANEL_GAP=7;
const PANEL_GAP_LINES=10;
const PANEL_GAP_COMPACT=6;
const SHELL_HORIZONTAL_INSET=16;
const modeSwitch=document.querySelector('.mode-switch');

let activeMode='daily';
let editing=false;
let selectedWidget=null;
let focusSeconds=25*60;
let focusRunning=false;
function panelGapFor(mode=activeMode){
  const state=profileState(mode);
  const h=Number(state.dockHeight)||DEFAULT_DOCK_HEIGHT;
  if(h<=80)return PANEL_GAP_COMPACT;
  return state.appearance==='lines'?PANEL_GAP_LINES:PANEL_GAP;
}


let focusInterval=null;
let panSession=null;
let dragSession=null;
let resizeSession=null;

const widgetsFor=mode=>[...document.querySelector(`.dock-panel[data-panel="${mode}"]`).children].filter(el=>el.classList.contains('widget'));

function cleanText(value,maxLength){
  return typeof value==='string'?value.replace(/[\u0000-\u001f\u007f]/g,' ').trim().slice(0,maxLength):'';
}
function loadCustomWidgets(){
  try{
    const saved=JSON.parse(localStorage.getItem(CUSTOM_STORAGE_KEY)||'null');
    if(saved?.version!==1||!Array.isArray(saved.widgets))return [];
    const seen=new Set();
    return saved.widgets.slice(0,MAX_CUSTOM_WIDGETS).flatMap(item=>{
      if(!item||!/^custom-[a-z0-9]{6,48}$/.test(item.id)||seen.has(item.id))return [];
      const label=cleanText(item.label,40),value=cleanText(item.value,32),detail=cleanText(item.detail,80);
      if(!label||!value)return [];
      seen.add(item.id);return [{id:item.id,label,value,detail}];
    });
  }catch{return [];}
}
const customWidgets=loadCustomWidgets();
function registerCustomWidget(definition){
  panels.forEach(panel=>{
    const widget=document.createElement('div');
    widget.className='tile widget custom-metric';
    Object.assign(widget.dataset,{widgetId:definition.id,widgetName:definition.label,defaultW:'190',minW:'96',maxW:'520',startHidden:'true'});
    widget.innerHTML='<button class="widget-drag" title="Drag to move; arrow keys to reorder" type="button">•••</button><div class="custom-metric-copy"><span class="custom-label"></span><b class="custom-value"></b><span class="custom-detail"></span></div><span class="widget-edit-label"></span><button class="widget-resize" title="Drag sideways; arrow keys to resize" type="button"></button>';
    widget.querySelector('.custom-label').textContent=definition.label;
    widget.querySelector('.custom-value').textContent=definition.value;
    widget.querySelector('.custom-detail').textContent=definition.detail;
    widget.querySelector('.widget-edit-label').textContent=definition.label;
    widget.querySelector('.widget-drag').setAttribute('aria-label',`Move ${definition.label}; arrow keys to reorder`);
    widget.querySelector('.widget-resize').setAttribute('aria-label',`Resize ${definition.label}; arrow keys change width`);
    panel.insertBefore(widget,panel.querySelector('.empty-profile'));
  });
}
// Definitions must exist before building defaults and merging persisted profile geometry.
customWidgets.forEach(registerCustomWidget);

const defaults={};
panels.forEach(panel=>{
  const mode=panel.dataset.panel;
  const widgets=widgetsFor(mode);
  defaults[mode]={
    order:widgets.map(widget=>widget.dataset.widgetId),
    hidden:widgets.filter(widget=>widget.dataset.startHidden==='true').map(widget=>widget.dataset.widgetId),
    appearance:'capsules',
    dockHeight:DEFAULT_DOCK_HEIGHT,
    sizes:Object.fromEntries(widgets.map(widget=>[
      widget.dataset.widgetId,
      {w:Number(widget.dataset.defaultW),h:DEFAULT_DOCK_HEIGHT-DOCK_VERTICAL_INSET}
    ]))
  };
});

const clone=value=>JSON.parse(JSON.stringify(value));
const clamp=(value,min,max)=>Math.min(max,Math.max(min,value));
const activePanel=()=>document.querySelector(`.dock-panel[data-panel="${activeMode}"]`);
const profileState=(mode=activeMode)=>dockState.profiles[mode];
const dockHeightFor=(mode=activeMode)=>clamp(Number(profileState(mode).dockHeight)||DEFAULT_DOCK_HEIGHT,MIN_DOCK_HEIGHT,MAX_DOCK_HEIGHT);
const tileHeightFor=(mode=activeMode)=>dockHeightFor(mode)-DOCK_VERTICAL_INSET;
const visibleWidgetIds=(mode=activeMode)=>profileState(mode).order.filter(id=>!profileState(mode).hidden.includes(id));

function loadState(){
  let saved={};
  try{saved=JSON.parse(localStorage.getItem(STORAGE_KEY)||'{}');}catch{}
  if(!saved||typeof saved!=='object'||Array.isArray(saved))saved={};
  const next={activeMode:saved.activeMode==='founder'?'founder':'daily',profiles:{}};
  Object.keys(defaults).forEach(mode=>{
    const base=defaults[mode];
    const candidate=saved.profiles?.[mode];
    const current=candidate&&typeof candidate==='object'&&!Array.isArray(candidate)?candidate:{};
    const validIds=value=>Array.isArray(value)?[...new Set(value.filter(id=>typeof id==='string'&&base.order.includes(id)))]:[];
    const priorOrder=validIds(current.order);
    const order=[...priorOrder,...base.order.filter(id=>!priorOrder.includes(id))];
    const hidden=Array.isArray(current.hidden)?validIds(current.hidden):[...base.hidden];
    const appearance=current.appearance==='lines'?'lines':'capsules';
    const dockHeight=clamp(Number.isFinite(current.dockHeight)?current.dockHeight:base.dockHeight,MIN_DOCK_HEIGHT,MAX_DOCK_HEIGHT);
    const sizes={};
    base.order.forEach(id=>{
      const widget=widgetsFor(mode).find(item=>item.dataset.widgetId===id);
      const width=current.sizes?.[id]?.w;
      sizes[id]={w:clamp(Number.isFinite(width)?width:base.sizes[id].w,Number(widget.dataset.minW),Number(widget.dataset.maxW)),h:dockHeight-DOCK_VERTICAL_INSET};
    });
    next.profiles[mode]={order,hidden,appearance,dockHeight,sizes};
  });
  return next;
}

let dockState=loadState();
activeMode=dockState.activeMode;

function saveState(){
  try{localStorage.setItem(STORAGE_KEY,JSON.stringify(dockState));}catch{}
}

function applyProfile(mode){
  const panel=document.querySelector(`.dock-panel[data-panel="${mode}"]`);
  const empty=panel.querySelector('.empty-profile');
  const map=new Map(widgetsFor(mode).map(widget=>[widget.dataset.widgetId,widget]));
  const tileHeight=tileHeightFor(mode);
  profileState(mode).order.forEach(id=>{
    const widget=map.get(id);
    if(widget)panel.insertBefore(widget,empty);
  });
  let visibleIndex=0;
  widgetsFor(mode).forEach(widget=>{
    const id=widget.dataset.widgetId;
    const size=profileState(mode).sizes[id];
    size.h=tileHeight;
    widget.style.setProperty('--widget-w',`${size.w}px`);
    widget.style.setProperty('--widget-h',`${tileHeight}px`);
    widget.hidden=profileState(mode).hidden.includes(id);
    widget.dataset.firstVisible=widget.hidden?'false':String(visibleIndex++===0);
  });
  panel.classList.toggle('profile-empty',widgetsFor(mode).every(widget=>widget.hidden));
}

function fitDock(resetScroll=false){
  const state=profileState();
  const dockHeight=dockHeightFor();
  const tileHeight=tileHeightFor();
  const visibleIds=visibleWidgetIds();
  const panelGap=panelGapFor();
  const extraVisualPad=state.appearance==='lines'?4:0;
  const contentWidth=visibleIds.reduce((total,id)=>total+(state.sizes[id]?.w||0),0)+Math.max(0,visibleIds.length-1)*panelGap+extraVisualPad;
  const naturalWidth=visibleIds.length?contentWidth+SHELL_HORIZONTAL_INSET:266;
  const maxWidth=Math.max(96,innerWidth-22);
  const nextWidth=Math.min(Math.max(96,naturalWidth),maxWidth);
  stage.style.setProperty('--dock-height',`${dockHeight}px`);
  stage.style.setProperty('--dock-h',`${dockHeight}px`);
  stage.style.setProperty('--tile-h',`${tileHeight}px`);
  stage.style.setProperty('--dock-radius',`${Math.round(clamp(dockHeight*.27,20,34))}px`);
  stage.style.setProperty('--tile-radius',`${Math.round(clamp(tileHeight*.21,12,24))}px`);
  stage.style.setProperty('--panel-gap',`${panelGap}px`);
  stage.classList.toggle('compact-height',dockHeight<=80);
  updateAppearanceControls();
  shell.style.setProperty('--dock-width',`${nextWidth}px`);
  if(resetScroll||track.scrollLeft>Math.max(0,contentWidth-nextWidth+SHELL_HORIZONTAL_INSET))track.scrollLeft=0;
  updateHeightControl();
  updateToolbarPosition();
}

function updateAppearanceControls(){
  const appearance=(profileState().appearance==='lines')?'lines':'capsules';
  stage.classList.toggle('lines-style',appearance==='lines');
  if(appearanceValue)appearanceValue.textContent=appearance==='lines'?'Lines':'Capsules';
  appearanceButtons.forEach(button=>{const on=button.dataset.appearance===appearance;button.classList.toggle('active',on);button.setAttribute('aria-pressed',String(on));});
}

function updateHeightControl(){
  const height=dockHeightFor();
  const progress=((height-MIN_DOCK_HEIGHT)/(MAX_DOCK_HEIGHT-MIN_DOCK_HEIGHT))*100;
  dockHeightRange.value=String(height);
  dockHeightRange.style.setProperty('--height-progress',`${progress}%`);
  dockHeightValue.textContent=`${height} px`;
}

function setDockHeight(value,{animate=true}={}){
  const next=clamp(Math.round(Number(value)/2)*2,MIN_DOCK_HEIGHT,MAX_DOCK_HEIGHT);
  if(next===profileState().dockHeight){updateHeightControl();return;}
  profileState().dockHeight=next;
  applyProfile(activeMode);
  fitDock();
  refreshSelectedDetails();
  renderManager();
  saveState();
  if(animate)animateDockPulse();
}


function renderGallery(){
  if(!widgetGallery)return;
  widgetGallery.innerHTML='';
  const ids=[];
  widgetsFor(activeMode).forEach(widget=>{ if(!ids.includes(widget.dataset.widgetId)) ids.push(widget.dataset.widgetId); });
  ids.forEach(id=>{
    const widget=widgetsFor(activeMode).find(item=>item.dataset.widgetId===id);
    if(!widget)return;
    const visible=!profileState().hidden.includes(id);
    const card=document.createElement('article');
    card.className='gallery-card';
    const title=escapeHTML(widget.dataset.widgetName);
    const size=profileState().sizes[id]||{w:Number(widget.dataset.defaultW)||120};
    const openaiSrc=widgetsFor(activeMode).find(w=>w.dataset.widgetId==='chatgpt')?.querySelector('.brand-logo')?.src||'';
    const gmailSrc=widgetsFor(activeMode).find(w=>w.dataset.widgetId==='gmail')?.querySelector('.gmail-primary-logo')?.src||'';
    let preview='';
    if(id==='chatgpt'){
      preview=`<div class="gallery-size-row">
        <div class="gallery-size-demo large"><img class="gallery-openai" alt="" src="${openaiSrc}"><span class="text">Ask Anything</span><div class="mini-actions"><span></span><span></span><span></span></div></div>
      </div><div class="gallery-size-row"><div class="gallery-size-demo medium"><img class="gallery-openai" alt="" src="${openaiSrc}"><span class="text">Ask Anything</span><div class="mini-actions"><span></span><span></span></div></div><div class="gallery-size-demo small"><img class="gallery-openai" alt="" src="${openaiSrc}"><span class="text">Ask Anything</span></div><div class="gallery-size-demo icon"><img class="gallery-openai" alt="" src="${openaiSrc}"></div></div>`;
    } else if(id==='gmail'){
      preview=`<div class="gallery-size-row">
        <div class="gallery-size-demo large"><img class="g-logo" alt="" src="${gmailSrc}"><span class="text">Starting some words of latest email</span><div class="mini-actions"><span></span><span></span><span></span></div></div>
      </div><div class="gallery-size-row"><div class="gallery-size-demo medium"><img class="g-logo" alt="" src="${gmailSrc}"><span class="text">Search in Emails</span><div class="mini-actions"><span></span><span></span></div></div><div class="gallery-size-demo small"><img class="g-logo" alt="" src="${gmailSrc}"><span class="text">Search</span></div><div class="gallery-size-demo icon"><img class="g-logo" alt="" src="${gmailSrc}"></div></div>`;
    } else if(id==='weather'){
      preview=`<div class="gallery-size-row">
        <div class="gallery-size-demo large weather-demo"><div class="weather-copy"><span class="city">Ahmedabad</span><span class="deg">38°</span></div><div class="mini-hours"><div class="mini-hour"><span>2pm</span><i class="mini-cloud"></i></div><div class="mini-hour"><span>3pm</span><i class="mini-cloud"></i></div><div class="mini-hour"><span>4pm</span><i class="mini-cloud"></i></div><div class="mini-hour"><span>5pm</span><i class="mini-cloud"></i></div></div></div>
      </div><div class="gallery-size-row"><div class="gallery-size-demo medium weather-demo"><div class="weather-copy"><span class="city">Ahmedabad</span><span class="deg">38°</span></div><div class="mini-hours"><div class="mini-hour"><span>2pm</span><i class="mini-cloud"></i></div><div class="mini-hour"><span>3pm</span><i class="mini-cloud"></i></div></div></div><div class="gallery-size-demo small weather-demo"><div class="weather-copy"><span class="city">Ahmedabad</span><span class="deg">38°</span></div></div></div>`;
    } else {
      preview=`<div class="gallery-size-row"><div class="gallery-size-demo large"><span class="text">${title}</span></div><div class="gallery-size-demo icon"><span class="text">•</span></div></div>`;
    }
    card.innerHTML=`<div class="gallery-card-head"><div class="gallery-card-copy"><b>${title}</b><small>Current width ${Math.round(size.w)} px · adaptive preview</small></div></div>${preview}<button type="button" class="${visible?'added':''}" data-gallery-toggle="${id}">${visible?'Shown in profile':'Add to profile'}</button>`;
    widgetGallery.appendChild(card);
  });
}


function renderServiceWidgetOptions(){
  if(!serviceWidgetOptions)return;
  const services=widgetsFor(activeMode).filter(widget=>['chatgpt','gmail','weather'].includes(widget.dataset.widgetId));
  serviceWidgetOptions.innerHTML='';
  services.forEach(widget=>{
    const id=widget.dataset.widgetId;
    const visible=!profileState().hidden.includes(id);
    let logo='';
    if(id==='chatgpt'){
      const src=widget.querySelector('.brand-logo')?.src||'';
      logo=`<img class="service-openai" alt="${escapeHTML(widget.dataset.widgetName)}" src="${src}">`;
    } else if(id==='gmail'){
      const src=widget.querySelector('.gmail-primary-logo')?.src||'';
      logo=`<img alt="${escapeHTML(widget.dataset.widgetName)}" src="${src}">`;
    } else {
      logo=`<span class="cloud" style="transform:scale(1.15)"></span>`;
    }
    const label=document.createElement('label');
    label.className='service-option';
    label.innerHTML=`<input type="checkbox" ${visible?'checked':''} data-service-toggle="${id}"><span class="service-option-logo">${logo}</span><span class="service-option-copy"><b>${escapeHTML(widget.dataset.widgetName)}</b><small>${id==='weather'?'Adaptive forecast widget':'Adaptive service widget'}</small><span class="service-option-state">${visible?'Shown':'Hidden'}</span></span>`;
    serviceWidgetOptions.appendChild(label);
  });
}

function renderManager(){
  updateAppearanceControls();
  renderServiceWidgetOptions();
  managerProfile.textContent=`${activeMode[0].toUpperCase()+activeMode.slice(1)} profile`;
  updateHeightControl();
  widgetOptions.innerHTML='';
  renderGallery();
  const map=new Map(widgetsFor(activeMode).map(widget=>[widget.dataset.widgetId,widget]));
  profileState().order.forEach(id=>{
    const widget=map.get(id);
    if(!widget||['chatgpt','gmail','weather'].includes(id))return;
    const checked=!profileState().hidden.includes(id);
    const size=profileState().sizes[id];
    const row=document.createElement('label');
    row.className='widget-option';
    row.innerHTML=`<input type="checkbox" ${checked?'checked':''} data-widget-toggle="${id}"><span class="widget-toggle"></span><span class="widget-option-name">${escapeHTML(widget.dataset.widgetName)}</span><span class="widget-option-size">${Math.round(size.w)}×${tileHeightFor()}</span>`;
    widgetOptions.appendChild(row);
  });
}

function selectWidget(widget){
  if(selectedWidget)selectedWidget.classList.remove('widget-selected');
  selectedWidget=widget&&widget.closest('.dock-panel.active')?widget:null;
  if(selectedWidget)selectedWidget.classList.add('widget-selected');
  refreshSelectedDetails();
}

function refreshSelectedDetails(){
  if(!selectedWidget||selectedWidget.hidden){
    selectedWidgetName.textContent='Select a widget';
    selectedWidgetSize.textContent='Click any card, then use the controls';
    layoutToolbar.classList.add('no-selection');
    layoutToolbar.querySelectorAll('[data-editor-action]').forEach(button=>button.disabled=true);
    return;
  }
  const size=profileState().sizes[selectedWidget.dataset.widgetId];
  selectedWidgetName.textContent=selectedWidget.dataset.widgetName;
  selectedWidgetSize.textContent=`${Math.round(size.w)} px wide · ${tileHeightFor()} px high`;
  layoutToolbar.classList.remove('no-selection');
  layoutToolbar.querySelectorAll('[data-editor-action]').forEach(button=>button.disabled=false);
}

function motion(element,keyframes,options={}){
  if(reduced.matches||!element?.animate)return null;
  return element.animate(keyframes,{fill:'both',...options});
}
function animateDockPulse(){
  shell.classList.remove('motion-pulse');
  void shell.offsetWidth;
  shell.classList.add('motion-pulse');
  setTimeout(()=>shell.classList.remove('motion-pulse'),560);
}
function animateProfileIn(panel){
  motion(panel,[{opacity:0,transform:'translateY(12px) scale(.975)',filter:'blur(5px)'},{opacity:1,transform:'translateY(0) scale(1)',filter:'blur(0)'}],{duration:420,easing:'cubic-bezier(.22,.8,.25,1)'});
  [...panel.querySelectorAll('.widget:not([hidden])')].forEach((widget,index)=>motion(widget,[{opacity:0,transform:'translateY(18px) scale(.92)'},{opacity:1,transform:'translateY(0) scale(1)'}],{duration:380,delay:index*32,easing:'cubic-bezier(.2,1.15,.35,1)'}));
}
function animateWidgetIn(widget){
  motion(widget,[{opacity:0,transform:'translateY(14px) scale(.82)',filter:'blur(4px)'},{opacity:1,transform:'translateY(-3px) scale(1.025)',filter:'blur(0)',offset:.72},{opacity:1,transform:'translateY(0) scale(1)',filter:'blur(0)'}],{duration:420,easing:'cubic-bezier(.2,1.15,.35,1)'});
}
function animateWidgetOut(widget,onFinish){
  const animation=motion(widget,[{opacity:1,transform:'translateY(0) scale(1)',filter:'blur(0)'},{opacity:0,transform:'translateY(10px) scale(.78)',filter:'blur(5px)'}],{duration:210,easing:'cubic-bezier(.4,0,1,1)'});
  if(animation)animation.finished.catch(()=>{}).then(onFinish);else onFinish();
}
function captureWidgetPositions(){
  return new Map(widgetsFor(activeMode).filter(widget=>!widget.hidden).map(widget=>[widget.dataset.widgetId,widget.getBoundingClientRect()]));
}
function animateWidgetReflow(before){
  requestAnimationFrame(()=>widgetsFor(activeMode).filter(widget=>!widget.hidden).forEach(widget=>{
    const first=before.get(widget.dataset.widgetId);if(!first)return;
    const last=widget.getBoundingClientRect();const dx=first.left-last.left;const dy=first.top-last.top;
    if(Math.abs(dx)<1&&Math.abs(dy)<1)return;
    motion(widget,[{transform:`translate(${dx}px,${dy}px) scale(.98)`},{transform:'translate(0,0) scale(1)'}],{duration:420,easing:'cubic-bezier(.2,1.15,.35,1)'});
  }));
}
function animateResize(widget,width){
  const current=profileState().sizes[widget.dataset.widgetId].w;
  setWidgetSize(widget,width);
  motion(widget,[{transform:'scaleX(1)'},{transform:`scaleX(${width>current?1.025:.975})`,offset:.55},{transform:'scaleX(1)'}],{duration:300,easing:'cubic-bezier(.2,1.15,.35,1)'});
  animateDockPulse();
}
function setWidgetVisibility(id,visible){
  const widget=widgetsFor(activeMode).find(item=>item.dataset.widgetId===id);if(!widget)return;
  const before=captureWidgetPositions();
  const state=profileState();
  state.hidden=state.hidden.filter(item=>item!==id);
  if(!visible)state.hidden.push(id);
  applyProfile(activeMode);animateWidgetReflow(before);
  if(visible)requestAnimationFrame(()=>{if(!widget.hidden)animateWidgetIn(widget);});
  if(selectedWidget===widget&&!visible)selectWidget(widgetsFor(activeMode).find(item=>!item.hidden)||null);
  saveState();renderManager();fitDock();animateDockPulse();
}

function setEditing(value){
  finishPointerInteraction();
  editing=value;
  syncEditorAccess();
  stage.classList.toggle('editing',editing);
  layoutToolbar.classList.toggle('open',editing);
  addButton.classList.toggle('editing',editing);
  addButton.textContent=editing?'Done':'Customize';
  addButton.setAttribute('aria-label',editing?'Finish editing layout':'Customize dock');
  popover.classList.remove('open');
  resetMagnification();
  animateDockPulse();
  if(editing){
    motion(layoutToolbar,[{opacity:0,transform:'translateX(-50%) translateY(12px) scale(.94)'},{opacity:1,transform:'translateX(-50%) translateY(0) scale(1)'}],{duration:340,easing:'cubic-bezier(.2,1.15,.35,1)'});
    const first=widgetsFor(activeMode).find(widget=>!widget.hidden);
    selectWidget(first||null);
  }else{
    selectWidget(null);
  }
  fitDock();
}

function switchMode(mode){
  if(!defaults[mode]||mode===activeMode)return;
  finishPointerInteraction();
  const outgoing=activePanel();
  motion(outgoing,[{opacity:1,transform:'translateY(0) scale(1)',filter:'blur(0)'},{opacity:0,transform:'translateY(8px) scale(.985)',filter:'blur(4px)'}],{duration:150,easing:'ease-in'});
  selectWidget(null);
  activeMode=mode;
  dockState.activeMode=mode;
  saveState();
  panels.forEach(panel=>{const on=panel.dataset.panel===mode;panel.classList.toggle('active',on);panel.inert=!on;});
  modeButtons.forEach(button=>{
    const on=button.dataset.mode===mode;
    button.classList.toggle('active',on);
    button.setAttribute('aria-selected',String(on));
    button.tabIndex=on?0:-1;
  });
  applyProfile(mode);
  renderManager();
  requestAnimationFrame(()=>{
    fitDock(true);
    animateProfileIn(activePanel());
    animateDockPulse();
    if(editing)selectWidget(widgetsFor(activeMode).find(widget=>!widget.hidden)||null);
  });
}

function updateToolbarPosition(){
  if(!editing)return;
  const rect=shell.getBoundingClientRect();
  layoutToolbar.style.maxWidth=`${Math.min(720,innerWidth-20)}px`;
  layoutToolbar.style.bottom=`${Math.max(99,innerHeight-rect.top+12)}px`;
}

function setWidgetSize(widget,width,{persist=true}={}){
  if(!widget)return;
  const id=widget.dataset.widgetId;
  const size=profileState().sizes[id];
  size.w=clamp(Math.round(width),Number(widget.dataset.minW),Number(widget.dataset.maxW));
  size.h=tileHeightFor();
  widget.style.setProperty('--widget-w',`${size.w}px`);
  widget.style.setProperty('--widget-h',`${size.h}px`);
  refreshSelectedDetails();
  fitDock();
  if(persist){renderManager();saveState();}
}

function moveSelected(direction){
  if(!selectedWidget)return;
  const before=captureWidgetPositions();
  const state=profileState();
  const id=selectedWidget.dataset.widgetId;
  const visible=visibleWidgetIds();
  const index=visible.indexOf(id);
  const target=visible[clamp(index+direction,0,visible.length-1)];
  if(!target||target===id)return;
  const from=state.order.indexOf(id),to=state.order.indexOf(target);
  [state.order[from],state.order[to]]=[state.order[to],state.order[from]];
  applyProfile(activeMode);
  animateWidgetReflow(before);
  animateDockPulse();
  renderManager();saveState();fitDock();
}

function hideSelected(){
  if(selectedWidget)setWidgetVisibility(selectedWidget.dataset.widgetId,false);
}

modeButtons.forEach(button=>button.addEventListener('click',()=>switchMode(button.dataset.mode)));

addButton.addEventListener('click',event=>{
  event.stopPropagation();
  if(editing){setEditing(false);return;}
  renderManager();
  popover.classList.toggle('open');
});
managerClose.addEventListener('click',()=>popover.classList.remove('open'));
editLayoutButton.addEventListener('click',()=>setEditing(true));
doneEditingButton.addEventListener('click',()=>setEditing(false));
manageWidgetsButton.addEventListener('click',event=>{
  event.stopPropagation();
  renderManager();
  popover.classList.toggle('open');
});
document.addEventListener('click',event=>{
  if(!event.target.closest('.settings-popover')&&!event.target.closest('#modeAdd')&&!event.target.closest('#manageWidgets'))popover.classList.remove('open');
});


serviceWidgetOptions?.addEventListener('change',event=>{
  const input=event.target.closest('[data-service-toggle]');
  if(!input)return;
  setWidgetVisibility(input.dataset.serviceToggle,input.checked);
});

widgetOptions.addEventListener('change',event=>{
  const input=event.target.closest('[data-widget-toggle]');
  if(!input)return;
  setWidgetVisibility(input.dataset.widgetToggle,input.checked);
});

widgetGallery?.addEventListener('click',event=>{
  const button=event.target.closest('[data-gallery-toggle]');
  if(!button)return;
  const id=button.dataset.galleryToggle;
  const shouldShow=profileState().hidden.includes(id);
  setWidgetVisibility(id,shouldShow);
  if(shouldShow){
    const widget=widgetsFor(activeMode).find(item=>item.dataset.widgetId===id);
    if(widget)selectWidget(widget);
  }
});

appearanceButtons.forEach(button=>button.addEventListener('click',()=>{
  const next=button.dataset.appearance;
  if(profileState().appearance===next)return;
  profileState().appearance=next;
  updateAppearanceControls();
  fitDock();
  saveState();
  animateDockPulse();
}));

dockHeightRange.addEventListener('input',()=>setDockHeight(dockHeightRange.value,{animate:false}));
dockHeightRange.addEventListener('change',()=>animateDockPulse());
showAllButton.addEventListener('click',()=>{
  profileState().hidden=[];
  applyProfile(activeMode);
  saveState();
  renderManager();
  fitDock();
});
resetLayoutButton.addEventListener('click',()=>{
  dockState.profiles[activeMode]=clone(defaults[activeMode]);
  applyProfile(activeMode);
  saveState();
  renderManager();
  fitDock(true);
  if(editing)selectWidget(widgetsFor(activeMode).find(widget=>!widget.hidden)||null);
});

layoutToolbar.addEventListener('click',event=>{
  const dockAction=event.target.closest('[data-dock-action]')?.dataset.dockAction;
  if(dockAction==='shorter')setDockHeight(dockHeightFor()-4);
  if(dockAction==='taller')setDockHeight(dockHeightFor()+4);
  const action=event.target.closest('[data-editor-action]')?.dataset.editorAction;
  if(!action)return;
  if(action==='left')moveSelected(-1);
  if(action==='right')moveSelected(1);
  if(action==='narrow'&&selectedWidget){const size=profileState().sizes[selectedWidget.dataset.widgetId];animateResize(selectedWidget,size.w-20);}
  if(action==='wide'&&selectedWidget){const size=profileState().sizes[selectedWidget.dataset.widgetId];animateResize(selectedWidget,size.w+20);}
  if(action==='hide')hideSelected();
});

activePanel().addEventListener('click',()=>{});
document.addEventListener('click',event=>{
  if(!editing)return;
  const widget=event.target.closest('.dock-panel.active .widget');
  if(widget&&!event.target.closest('.widget-drag,.widget-resize,.app-button,.play'))selectWidget(widget);
});

function pointerPoint(event){return {x:event.clientX,y:event.clientY};}

document.addEventListener('pointerdown',event=>{
  if(!editing)return;
  const resizeHandle=event.target.closest('.widget-resize');
  if(resizeHandle){
    event.preventDefault();event.stopPropagation();
    const widget=resizeHandle.closest('.widget');
    selectWidget(widget);
    const size=profileState().sizes[widget.dataset.widgetId];
    resizeSession={pointerId:event.pointerId,widget,start:pointerPoint(event),startW:size.w};
    resizeHandle.setPointerCapture?.(event.pointerId);
    document.body.classList.add('resizing-widget');
    return;
  }
  const dragHandle=event.target.closest('.widget-drag');
  if(!dragHandle)return;
  event.preventDefault();event.stopPropagation();
  const widget=dragHandle.closest('.widget');
  selectWidget(widget);
  const panel=widget.parentElement;
  const rect=widget.getBoundingClientRect();
  const placeholder=document.createElement('div');
  placeholder.className='widget-placeholder';
  placeholder.style.setProperty('--placeholder-w',`${rect.width}px`);
  placeholder.style.setProperty('--placeholder-h',`${rect.height}px`);
  panel.insertBefore(placeholder,widget);
  widget.classList.add('widget-floating');
  document.body.appendChild(widget);
  Object.assign(widget.style,{position:'fixed',left:`${rect.left}px`,top:`${rect.top}px`,zIndex:'9999',pointerEvents:'none'});
  dragSession={pointerId:event.pointerId,widget,panel,placeholder,offsetX:event.clientX-rect.left,offsetY:event.clientY-rect.top,before:captureWidgetPositions()};
  dragHandle.setPointerCapture?.(event.pointerId);
  document.body.classList.add('dragging-widget');
});

document.addEventListener('pointermove',event=>{
  if(resizeSession){
    const {widget,start,startW}=resizeSession;
    setWidgetSize(widget,startW+(event.clientX-start.x),{persist:false});
    return;
  }
  if(!dragSession)return;
  const {widget,panel,placeholder,offsetX,offsetY}=dragSession;
  widget.style.left=`${event.clientX-offsetX}px`;
  widget.style.top=`${event.clientY-offsetY}px`;
  const shellRect=shell.getBoundingClientRect();
  if(event.clientX<shellRect.left+45)track.scrollLeft-=14;
  if(event.clientX>shellRect.right-45)track.scrollLeft+=14;
  const candidates=[...panel.children].filter(element=>element.classList.contains('widget')&&!element.hidden&&element!==widget);
  let placed=false;
  for(const candidate of candidates){
    const rect=candidate.getBoundingClientRect();
    if(event.clientX<rect.left+rect.width/2){panel.insertBefore(placeholder,candidate);placed=true;break;}
  }
  if(!placed)panel.insertBefore(placeholder,panel.querySelector('.empty-profile'));
});

function finishPointerInteraction(){
  if(resizeSession){
    resizeSession=null;
    document.body.classList.remove('resizing-widget');
    saveState();renderManager();fitDock();
  }
  if(dragSession){
    const {widget,placeholder,before}=dragSession;
    placeholder.replaceWith(widget);
    widget.classList.remove('widget-floating');
    ['position','left','top','z-index','pointer-events'].forEach(property=>widget.style.removeProperty(property));
    profileState().order=widgetsFor(activeMode).map(item=>item.dataset.widgetId);
    dragSession=null;
    document.body.classList.remove('dragging-widget');
    animateWidgetReflow(before);
    motion(widget,[{transform:'translateY(-6px) scale(1.045)'},{transform:'translateY(0) scale(1)'}],{duration:360,easing:'cubic-bezier(.2,1.15,.35,1)'});
    animateDockPulse();
    saveState();renderManager();fitDock();selectWidget(widget);
  }
}
document.addEventListener('pointerup',finishPointerInteraction);
document.addEventListener('pointercancel',finishPointerInteraction);

function updateClock(){
  const now=new Date();
  document.querySelector('#timeMain').textContent=now.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit',hour12:false});
  document.querySelector('#timeDate').textContent=now.toLocaleDateString([],{weekday:'short',month:'short',day:'numeric'});
  document.querySelector('#menuClock').textContent=now.toLocaleTimeString([],{hour:'2-digit',minute:'2-digit'});
  const clock=document.querySelector('#analogClock');
  const h=now.getHours()%12,m=now.getMinutes(),s=now.getSeconds();
  clock.querySelector('.hour').style.transform=`rotate(${h*30+m*.5}deg)`;
  clock.querySelector('.minute').style.transform=`rotate(${m*6}deg)`;
  clock.querySelector('.second').style.transform=`rotate(${s*6}deg)`;
}
const analog=document.querySelector('#analogClock');
for(let i=0;i<12;i++){const tick=document.createElement('span');tick.className='tick';tick.style.transform=`rotate(${i*30}deg)`;analog.appendChild(tick);}
['hour','minute','second'].forEach(name=>{const hand=document.createElement('span');hand.className=`hand ${name}`;analog.appendChild(hand);});
updateClock();setInterval(updateClock,1000);

function updateEnergy(){
  const total=30*60;
  const left=total-Math.floor((Date.now()/1000)%total);
  document.querySelector('#energyTimer').textContent=`${String(Math.floor(left/60)).padStart(2,'0')}:${String(left%60).padStart(2,'0')}`;
}
updateEnergy();setInterval(updateEnergy,1000);

function renderFocus(){document.querySelector('#focusTimer').textContent=`${String(Math.floor(focusSeconds/60)).padStart(2,'0')}:${String(focusSeconds%60).padStart(2,'0')}`;}
document.querySelector('#playFocus').addEventListener('click',()=>{
  focusRunning=!focusRunning;
  document.querySelector('#playFocus').textContent=focusRunning?'Ⅱ':'▶';
  clearInterval(focusInterval);
  if(focusRunning)focusInterval=setInterval(()=>{
    focusSeconds=Math.max(0,focusSeconds-1);renderFocus();
    if(!focusSeconds){focusRunning=false;clearInterval(focusInterval);document.querySelector('#playFocus').textContent='▶';}
  },1000);
});
renderFocus();

function resetMagnification(){
  document.querySelectorAll('.app-button').forEach(item=>{
    item.style.setProperty('--scale','1');item.style.setProperty('--lift','0px');item.style.zIndex=1;
  });
}
track.addEventListener('pointermove',event=>{
  if(editing||reduced.matches||panSession)return;
  document.querySelectorAll('.dock-panel.active .apps-widget:not([hidden]) .app-button').forEach(item=>{
    const rect=item.getBoundingClientRect();
    const distance=Math.abs(event.clientX-(rect.left+rect.width/2));
    const influence=Math.max(0,1-distance/105);
    const eased=influence*influence*(3-2*influence);
    item.style.setProperty('--scale',(1+eased*.40).toFixed(3));
    item.style.setProperty('--lift',`${-eased*22}px`);
    item.style.zIndex=2+Math.round(eased*10);
  });
});
track.addEventListener('pointerleave',resetMagnification);
track.addEventListener('wheel',event=>{
  if(track.scrollWidth>track.clientWidth){event.preventDefault();track.scrollLeft+=event.deltaY+event.deltaX;}
},{passive:false});
track.addEventListener('pointerdown',event=>{
  if(editing||event.target.closest('button'))return;
  panSession={pointerId:event.pointerId,startX:event.clientX,startScroll:track.scrollLeft};
  track.setPointerCapture?.(event.pointerId);
  track.classList.add('dragging');resetMagnification();
});
track.addEventListener('pointermove',event=>{if(panSession)track.scrollLeft=panSession.startScroll-(event.clientX-panSession.startX);});
track.addEventListener('pointerup',()=>{panSession=null;track.classList.remove('dragging');});
track.addEventListener('pointercancel',()=>{panSession=null;track.classList.remove('dragging');});

window.addEventListener('resize',()=>fitDock());
document.addEventListener('keydown',event=>{
  if(event.key==='Escape'){
    finishPointerInteraction();
    if(popover.classList.contains('open')){popover.classList.remove('open');addButton.focus();}
    else if(editing){setEditing(false);addButton.focus();}
  }
});


function syncEditorAccess(){
  panels.forEach(panel=>panel.querySelectorAll('.widget').forEach(widget=>{
    widget.tabIndex=editing?0:-1;
    widget.setAttribute('aria-label',widget.dataset.widgetName);
    widget.querySelectorAll('.widget-drag,.widget-resize').forEach(button=>{button.tabIndex=editing?0:-1;button.disabled=!editing;});
  }));
}
modeButtons.forEach((button,index)=>button.addEventListener('keydown',event=>{
  let next=index;
  if(event.key==='ArrowRight')next=(index+1)%modeButtons.length;
  else if(event.key==='ArrowLeft')next=(index+modeButtons.length-1)%modeButtons.length;
  else if(event.key==='Home')next=0;
  else if(event.key==='End')next=modeButtons.length-1;
  else return;
  event.preventDefault();switchMode(modeButtons[next].dataset.mode);modeButtons[next].focus();
}));
document.addEventListener('keydown',event=>{
  if(!editing)return;
  const widget=event.target.closest('.dock-panel.active .widget');
  if(!widget)return;
  if(event.target===widget&&(event.key==='Enter'||event.key===' ')){event.preventDefault();selectWidget(widget);}
  if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown'].includes(event.key))return;
  if(event.target.closest('.widget-drag')){event.preventDefault();selectWidget(widget);moveSelected(event.key==='ArrowLeft'||event.key==='ArrowUp'?-1:1);}
  if(event.target.closest('.widget-resize')){event.preventDefault();selectWidget(widget);const step=event.shiftKey?20:10;setWidgetSize(widget,profileState().sizes[widget.dataset.widgetId].w+(event.key==='ArrowLeft'||event.key==='ArrowDown'?-step:step));}
});
document.addEventListener('focusin',event=>{
  if(editing){const widget=event.target.closest('.dock-panel.active .widget');if(widget)selectWidget(widget);}
});
new MutationObserver(()=>addButton.setAttribute('aria-expanded',String(popover.classList.contains('open')))).observe(popover,{attributes:true,attributeFilter:['class']});
document.querySelector('#customWidgetForm').addEventListener('submit',event=>{
  event.preventDefault();
  const status=document.querySelector('#customWidgetStatus');
  if(customWidgets.length>=MAX_CUSTOM_WIDGETS){status.textContent='The 12 custom widget limit has been reached.';return;}
  const label=cleanText(document.querySelector('#customLabel').value,40);
  const value=cleanText(document.querySelector('#customValue').value,32);
  const detail=cleanText(document.querySelector('#customDetail').value,80);
  if(!label||!value){status.textContent='Enter a label and value.';return;}
  const random=globalThis.crypto?.getRandomValues?Array.from(crypto.getRandomValues(new Uint32Array(2)),n=>n.toString(36)).join(''):Date.now().toString(36)+customWidgets.length.toString(36);
  const definition={id:`custom-${random}`,label,value,detail};
  const next=[...customWidgets,definition];
  try{localStorage.setItem(CUSTOM_STORAGE_KEY,JSON.stringify({version:1,widgets:next}));}catch{status.textContent='Browser storage is unavailable. The widget could not be saved.';return;}
  customWidgets.push(definition);registerCustomWidget(definition);
  Object.keys(defaults).forEach(mode=>{
    const base=defaults[mode],state=profileState(mode);
    base.order.push(definition.id);base.hidden.push(definition.id);base.sizes[definition.id]={w:190,h:DEFAULT_DOCK_HEIGHT-DOCK_VERTICAL_INSET};
    state.order.push(definition.id);state.sizes[definition.id]={w:190,h:tileHeightFor(mode)};
    if(mode!==activeMode)state.hidden.push(definition.id);
    applyProfile(mode);
  });
  syncEditorAccess();saveState();renderManager();fitDock();
  event.target.reset();status.textContent=`Added ${label} to this profile. It is also available in the other profile's gallery.`;
});
syncEditorAccess();

Object.keys(defaults).forEach(applyProfile);
panels.forEach(panel=>{const on=panel.dataset.panel===activeMode;panel.classList.toggle('active',on);panel.inert=!on;});
modeButtons.forEach(button=>{
  const on=button.dataset.mode===activeMode;
  button.classList.toggle('active',on);
  button.setAttribute('aria-selected',String(on));
    button.tabIndex=on?0:-1;
});
renderManager();
requestAnimationFrame(()=>{
  fitDock(true);
  motion(stage,[{opacity:0,transform:'translateX(-50%) translateY(30px) scale(.94)',filter:'blur(8px)'},{opacity:1,transform:'translateX(-50%) translateY(-3px) scale(1.01)',filter:'blur(0)',offset:.78},{opacity:1,transform:'translateX(-50%) translateY(0) scale(1)',filter:'blur(0)'}],{duration:680,easing:'cubic-bezier(.2,1.05,.3,1)'});
  motion(modeSwitch,[{opacity:0,transform:'translateY(10px) scale(.9)'},{opacity:1,transform:'translateY(0) scale(1)'}],{duration:480,delay:160,easing:'cubic-bezier(.2,1.15,.35,1)'});
  animateProfileIn(activePanel());
});


})();
