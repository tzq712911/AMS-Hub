let privateLibrary = {entries: []};
let privateLibraryLoaded = false;
let rfidBindings = [];
let rfidBindingsLoaded = false;
let librarySaving = false;
const libraryField = id => document.getElementById(id);
function libraryStatus(text) { libraryField('library-status').textContent = text; }
let libraryCategory='';
function normalizeLibraryColor(value){const s=(value||'').trim().replace(/^#/,'').toUpperCase();return /^[0-9A-F]{6}$/.test(s)?s+'FF':/^[0-9A-F]{8}$/.test(s)?s:'';}
function entryForBinding(binding,entries=privateLibrary.entries){
 const same=e=>e.brand===binding.brand&&e.name===(binding.name||binding.material)&&e.material===binding.material;
 return entries.find(e=>e.code===binding.preset_code&&e.material===binding.material)
  ||entries.find(e=>same(e)&&normalizeLibraryColor(e.color)&&normalizeLibraryColor(e.color)===normalizeLibraryColor(binding.color))
  ||entries.find(e=>same(e)&&!e.color);
}
function visibleLibraryEntries(entries,bindings,{groupBy='none',category='',search=''}={}){
 const q=search.trim().toLocaleLowerCase().replace(/^#/,'');
 return entries.filter(e=>(!category||!['brand','material'].includes(groupBy)||e[groupBy]===category)&&(!q||[e.brand,e.name,e.material,e.color||'',e.color_name||'',...bindings.filter(b=>entryForBinding(b,entries)?.code===e.code).map(b=>b.tag_id)].join(' ').toLocaleLowerCase().includes(q)));
}
function materialFamilyKey(e){return JSON.stringify([e.brand,e.name,e.material]);}
function materialFamilies(entries){const groups=new Map();for(const e of entries){const key=materialFamilyKey(e);if(!groups.has(key))groups.set(key,[]);groups.get(key).push(e);}return [...groups.values()];}
function familyBindings(variants,bindings,entries=privateLibrary.entries){const codes=new Set(variants.map(e=>e.code));return bindings.filter(b=>codes.has(entryForBinding(b,entries)?.code));}
let webBrandCatalog=[];
function brandSuggestions(entries,catalog=webBrandCatalog){return [...new Set(['Bambu Lab','Generic','鑫博川','eSUN','Polymaker','SUNLU','Kexcelled','Creality','Overture','Anycubic','Snapmaker',...entries.map(e=>e.brand),...catalog].map(s=>s==='Bambu'?'Bambu Lab':s.trim()).filter(Boolean))];}
function populateBrandSuggestions(){const list=libraryField('library-brand-options');if(!list||typeof list.append!=='function')return;list.replaceChildren();for(const brand of brandSuggestions(privateLibrary.entries)){const option=document.createElement('option');option.value=brand;list.append(option);}}
async function loadBrandSuggestions(){try{const response=await fetch('/filament-brands');if(!response.ok)return;webBrandCatalog=(await response.text()).split(/\r?\n/).filter(s=>s.trim()&&!s.startsWith('#')).map(s=>s.split(',')[0]);populateBrandSuggestions();}catch{}}
function updateLibraryColor(){
 const preview=libraryField('library-color-preview');if(!preview||!preview.style)return;
 const value=normalizeLibraryColor(libraryField('library-color').value);preview.classList.toggle('unset',!value);preview.style.background=value?'#'+value.slice(0,6):'';
 libraryField('library-color-preview-label').textContent=value?(libraryField('library-color_name').value.trim()||'#'+value.slice(0,6)):'请选择颜色';
 if(value)libraryField('library-color-picker').value='#'+value.slice(0,6);
 const swatches=libraryField('library-color-swatches');if(swatches?.querySelectorAll)swatches.querySelectorAll('button').forEach(b=>b.classList.toggle('selected',b.dataset.color===value));
}
function setLibraryColor(value,name){const color=normalizeLibraryColor(value);libraryField('library-color').value=color;if(name!==undefined)libraryField('library-color_name').value=name;updateLibraryColor();}
function initLibraryColorControls(){
 const root=libraryField('library-color-swatches');if(!root||typeof root.append!=='function')return;
 for(const [name,rgb]of [['白色','FFFFFF'],['黑色','000000'],['灰色','969696'],['红色','F01B23'],['橙色','FF8B3D'],['黄色','F5D14B'],['绿色','00CE9B'],['青色','71CCE1'],['蓝色','287FC2'],['紫色','A083CC'],['粉色','E383BA'],['棕色','805838']]){
  const button=libraryNode('button','library-color-swatch');button.type='button';button.style.background='#'+rgb;button.dataset.color=rgb+'FF';button.title=name+' #'+rgb;button.setAttribute('aria-label',name);button.addEventListener('click',()=>setLibraryColor(rgb,name));root.append(button);
 }
 libraryField('library-color-picker').addEventListener('input',e=>setLibraryColor(e.target.value,''));
 libraryField('library-color').addEventListener('input',updateLibraryColor);libraryField('library-color_name').addEventListener('input',updateLibraryColor);
 const group=libraryField('library-group-by');group.addEventListener('change',()=>{libraryCategory='';renderLibraryCards();});updateLibraryColor();
}
function libraryNode(tag,className,text){const n=document.createElement(tag);if(className)n.className=className;if(text!==undefined)n.textContent=text;return n;}
function bindingColor(value){return /^[\da-f]{8}$/i.test(value||'')?'#'+value.slice(0,6):'#555555';}
function appendBinding(container,b){const row=libraryNode('span','rfid-chip');const swatch=libraryNode('i','rfid-swatch');swatch.style.background=bindingColor(b.color);row.append(swatch,libraryNode('span','',b.tag_id));row.title=`${b.brand} / ${b.name||b.material} / ${b.material} · 库存 ${b.id}`;container.append(row);}
function populateLibrary() {
 populateBrandSuggestions();
 const select=libraryField('library-entry'),current=select.value;select.replaceChildren(new Option('新建耗材档案',''));
 for (const e of privateLibrary.entries) select.add(new Option(`${e.brand} / ${e.name} / ${e.material}`,e.code));
 if(privateLibrary.entries.some(e=>e.code===current))select.value=current;
 renderLibraryCards();
 if(typeof document.dispatchEvent==='function')document.dispatchEvent(new Event('library-changed'));
}
function materialCard(variants,bindings,current){
 const e=variants.find(e=>e.code===current)||variants[0];
 const card=libraryNode('button','saved-material-card');card.type='button';card.classList.toggle('active',current===e.code);card.setAttribute('aria-pressed',current===e.code?'true':'false');
 const own=normalizeLibraryColor(e.color),actual=[...new Set(bindings.map(b=>normalizeLibraryColor(b.color)).filter(Boolean))],colors=[...new Set([...variants.map(v=>normalizeLibraryColor(v.color)).filter(Boolean),...actual])];
 const color=libraryNode('span','material-card-color'+(!colors.length?' unset':''));
 if(colors.length===1)color.style.background='#'+colors[0].slice(0,6);else if(colors.length>1)color.style.background='linear-gradient(90deg,'+colors.flatMap((c,i)=>['#'+c.slice(0,6)+' '+i/colors.length*100+'%','#'+c.slice(0,6)+' '+(i+1)/colors.length*100+'%']).join(',')+')';
 color.append(libraryNode('span','',colors.length>1?colors.length+' 色 · '+bindings.length+' 个 RFID':own?(e.color_name||'#'+own.slice(0,6)):actual.length?'RFID · '+actual.length+' 色':'未设置颜色'));card.append(color);
 const header=libraryNode('span','material-card-header');const identity=libraryNode('span','material-card-identity');identity.append(libraryNode('small','',e.brand),libraryNode('strong','',e.name));header.append(identity,libraryNode('span','material-type-badge',e.material));card.append(header);
 const maps=libraryNode('span','material-maps');for(const text of [`拓竹 · ${e.bambu_code||'Generic '+e.material}`,`U1 · ${e.u1_vendor||'Generic'} ${e.u1_subtype||e.material}`,`ACE · ${e.material}`]){const item=libraryNode('span','',text);item.title=text;maps.append(item);}card.append(maps);
 const ids=libraryNode('span','material-rfids');if(bindings.length){for(const b of bindings)appendBinding(ids,b);}else ids.append(libraryNode('span','no-rfid',rfidBindingsLoaded?'未绑定 RFID':'RFID 关联未读取'));card.append(ids);
 card.addEventListener('click',()=>{if(librarySaving)return;libraryField('library-entry').value=e.code;editPrivateEntry();if(innerWidth<701)libraryField('material-editor-title').scrollIntoView({behavior:'smooth'});});return card;
}
function renderLibraryCards(){
 const list=libraryField('saved-material-list');if(!list||typeof list.append!=='function')return;
 const current=libraryField('library-entry').value,search=libraryField('library-search').value.trim(),groupBy=libraryField('library-group-by').value;
 const categorized=['brand','material'].includes(groupBy),categories=[...new Set(privateLibrary.entries.map(e=>e[groupBy]).filter(Boolean))].sort((a,b)=>a.localeCompare(b,'zh-CN'));
 if(!categories.includes(libraryCategory))libraryCategory='';const tabs=libraryField('library-category-tabs');tabs.hidden=!categorized;tabs.replaceChildren();
 if(categorized)for(const category of ['',...categories]){const count=category?privateLibrary.entries.filter(e=>e[groupBy]===category).length:privateLibrary.entries.length;const button=libraryNode('button','library-category-chip',(category||'全部')+' '+count);button.type='button';button.classList.toggle('active',category===libraryCategory);button.addEventListener('click',()=>{libraryCategory=category;renderLibraryCards();});tabs.append(button);}
 const visible=visibleLibraryEntries(privateLibrary.entries,rfidBindings,{groupBy,category:libraryCategory,search});list.replaceChildren();libraryField('saved-material-count').textContent=`显示 ${visible.length} / ${privateLibrary.entries.length} 个耗材档案`;
 const buckets=new Map();for(const e of visible){const key=categorized?e[groupBy]:'';if(!buckets.has(key))buckets.set(key,[]);buckets.get(key).push(e);}
 for(const [name,entries]of [...buckets].sort((a,b)=>a[0].localeCompare(b[0],'zh-CN'))){const section=libraryNode('section','material-group');if(name)section.append(libraryNode('h3','material-group-title',name+' · '+entries.length));const grid=libraryNode('div','material-card-grid');for(const family of materialFamilies(privateLibrary.entries).filter(f=>f.some(e=>entries.includes(e))))grid.append(materialCard(family,familyBindings(family,rfidBindings),current));section.append(grid);list.append(section);}
 if(!visible.length)list.append(libraryNode('p','empty-state',privateLibrary.entries.length?'没有匹配的耗材或 RFID':'尚无私有耗材，点击“新增耗材”建立映射'));
 const q=search.toLocaleLowerCase(),unlinked=rfidBindings.filter(b=>!entryForBinding(b)&&(!categorized||!libraryCategory||b[groupBy]===libraryCategory)&&(!q||[b.brand,b.name,b.material,b.tag_id].join(' ').toLocaleLowerCase().includes(q)));
 const unmapped=libraryField('unmapped-rfid-list');unmapped.replaceChildren();libraryField('unmapped-rfid-section').hidden=!unlinked.length;
 for(const b of unlinked){const card=libraryNode('button','unmapped-rfid-card');card.type='button';card.append(libraryNode('strong','',`${b.brand||'未记录品牌'} / ${b.name||b.material}`));appendBinding(card,b);card.append(libraryNode('small','',`${b.material} · 点击建立映射`));card.addEventListener('click',()=>{
  if(librarySaving)return;newPrivateEntry();for(const [k,v]of Object.entries({brand:b.brand,name:b.name||b.material,material:b.material,bambu_code:/^[GP]/.test(b.preset_code)?b.preset_code:''}))libraryField('library-'+k).value=v;
  setLibraryColor(b.color,'');libraryField('selected-rfid-info').textContent='来源 RFID：'+b.tag_id+'。保存后按品牌、类型与颜色匹配映射。';
 });unmapped.append(card);}
 updateSelectedBindings();
}
function updateSelectedBindings(){
 const box=libraryField('selected-rfid-info');if(!box||typeof box.append!=='function')return;
 const code=libraryField('library-entry').value;box.replaceChildren();const entry=privateLibrary.entries.find(e=>e.code===code);
 if(!entry){const variant=libraryField('library-color-variant');if(variant)variant.replaceChildren();box.textContent='新档案尚未关联 RFID';return;}
 const family=privateLibrary.entries.filter(e=>materialFamilyKey(e)===materialFamilyKey(entry));
 const variant=libraryField('library-color-variant');if(variant&&typeof variant.add==='function'){variant.replaceChildren();for(const e of family)variant.add(new Option((e.color_name||'颜色')+' · '+(e.color?'#'+e.color.slice(0,6):'未设置'),e.code));variant.value=code;}
 const linked=familyBindings(family,rfidBindings);box.append(libraryNode('strong','',`对应 RFID · ${rfidBindingsLoaded?linked.length:'未读取'}`));
 if(linked.length){const ids=libraryNode('div','material-rfids');for(const b of linked)appendBinding(ids,b);box.append(ids);}else box.append(libraryNode('p','hint',rfidBindingsLoaded?'此映射档案尚未绑定 RFID；扫描建档后会按现有资料匹配。':'请刷新 RFID 关联后查看。'));
}
async function loadRfidBindings(){
 try{const snapshot=[];let offset=0,total=0;
  do{const response=await fetch('/api/rfid-bindings?offset='+offset);if(!response.ok)throw Error(`读取失败 (${response.status})`);
   const data=JSON.parse(decrypt(getEncryptionKey(),await response.text()));if(data.error_text)throw Error(data.error_text);if(!Array.isArray(data.bindings))throw Error('RFID 响应不完整');
   snapshot.push(...data.bindings);total=data.total;const next=data.next_offset;if(next!=null&&(!Number.isInteger(next)||next<=offset))throw Error('RFID 分页无效');offset=next;
  }while(offset!=null);
  rfidBindings=[...new Map(snapshot.map(b=>[b.id+'|'+b.tag_id,b])).values()];rfidBindingsLoaded=true;
  libraryField('rfid-status').textContent=`已读取 ${rfidBindings.length} 个 RFID 档案 · 颜色来自实际耗材记录`;renderLibraryCards();return true;
 }catch(e){libraryField('rfid-status').textContent='RFID 关联读取失败：'+e.message+(rfidBindingsLoaded?'；保留上次关联显示':'');return false;}
}
async function refreshLibraryWorkspace(){const loaded=await loadPrivateLibrary();if(loaded)await loadRfidBindings();}
async function loadPrivateLibrary() {
 try {const response=await fetch('/api/filament-library');if(!response.ok)throw Error(`读取失败 (${response.status})`);
  const data=JSON.parse(decrypt(getEncryptionKey(),await response.text()));if(!Array.isArray(data.library?.entries))throw Error('耗材库响应不完整');
  privateLibrary=data.library;privateLibraryLoaded=true;populateLibrary();libraryStatus(`已读取 ${privateLibrary.entries.length} 条真实耗材档案`);return true;
 }catch(e){libraryStatus('请先验证密钥，再读取私有库：'+e.message);return false;}
}
function editPrivateEntry(){
 const e=privateLibrary.entries.find(e=>e.code===libraryField('library-entry').value)||{};
 for(const name of ['brand','name','material','color','color_name','bambu_code','bambu_setting','u1_vendor','u1_subtype'])libraryField('library-'+name).value=e[name]||(name==='material'?'PLA':name==='u1_vendor'?'Generic':'');
 libraryField('library-temp_min').value=e.temp_min||190;libraryField('library-temp_max').value=e.temp_max||240;
 libraryField('material-editor-title').textContent=e.code?'编辑耗材':'新增耗材';libraryField('delete-library-entry').disabled=!e.code;updateLibraryColor();renderLibraryCards();
}
function newPrivateEntry(){if(librarySaving)return;libraryField('library-entry').value='';editPrivateEntry();libraryStatus('填写实际耗材资料与映射后保存。');}
function collectPrivateEntry(){
 const current=libraryField('library-entry').value,e={code:current||'L'+Date.now().toString(16),temp_min:Number(libraryField('library-temp_min').value),temp_max:Number(libraryField('library-temp_max').value)};
 for(const name of ['brand','name','material','color_name','bambu_code','bambu_setting','u1_vendor','u1_subtype'])e[name]=libraryField('library-'+name).value.trim();
 e.color=normalizeLibraryColor(libraryField('library-color').value);if(!e.color)throw Error('请选择耗材颜色，或填写有效的 RGB / RGBA 颜色代码');
 if(!e.brand||!e.name||!e.material)throw Error('请填写实际品牌、耗材类型和准确的基础材料');if(!(e.temp_min>0&&e.temp_min<=e.temp_max&&e.temp_max<=500))throw Error('温度范围无效');return e;
}
async function savePrivateLibrary(next){
 const response=await sendData('/api/filament-library',{library:next});if(!response.ok)throw Error(`保存失败 (${response.status})`);
 const result=JSON.parse(decrypt(getEncryptionKey(),await response.text()));if(result.error_text)throw Error(result.error_text);
 privateLibrary=next;populateLibrary();libraryStatus(`已保存 ${next.entries.length} 条档案。机器映射随下次使用生效；RFID 真实资料保持不变。`);
}
async function savePrivateEntry(){
 if(librarySaving)return;librarySaving=true;
 try{if(!privateLibraryLoaded&&!await loadPrivateLibrary())return;const e=collectPrivateEntry();const entries=privateLibrary.entries.map(p=>p.code===e.code?e:p);if(!entries.some(p=>p.code===e.code))entries.push(e);await savePrivateLibrary({entries});libraryField('library-entry').value=e.code;const group=libraryField('library-group-by').value;if(['brand','material'].includes(group))libraryCategory=e[group];editPrivateEntry();}
 catch(e){libraryStatus(e.message);}finally{librarySaving=false;}
}
async function deletePrivateEntry(){
 if(librarySaving)return;const code=libraryField('library-entry').value;if(!code)return;
 if(!confirm('删除此耗材映射档案？已绑定的 RFID 真实档案会保留，后续使用同类型通用映射。'))return;
 librarySaving=true;try{await savePrivateLibrary({entries:privateLibrary.entries.filter(e=>e.code!==code)});libraryField('library-entry').value='';editPrivateEntry();}catch(e){libraryStatus(e.message);}finally{librarySaving=false;}
}
async function importPrivateLibrary(event){
 if(librarySaving){event.target.value='';return;}librarySaving=true;
 try{if(!privateLibraryLoaded&&!await loadPrivateLibrary())return;const data=JSON.parse(await event.target.files[0].text()),entries=(data.library||data).entries;if(!Array.isArray(entries))throw Error('请选择导出的私有库 JSON 文件');
  const merged=new Map(privateLibrary.entries.map(e=>[e.code,e]));for(const e of entries)merged.set(e.code,e);await savePrivateLibrary({entries:[...merged.values()]});
 }catch(e){libraryStatus('导入失败：'+e.message);}finally{librarySaving=false;event.target.value='';}
}
function exportPrivateLibrary(){
 if(!privateLibraryLoaded){libraryStatus('请先成功读取耗材库再导出。');return;}
 const blob=new Blob([JSON.stringify(privateLibrary,null,2)],{type:'application/json'}),url=URL.createObjectURL(blob),a=document.createElement('a');a.href=url;a.download='ams-hub-private-library.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
}
if(typeof libraryField('library-search')?.addEventListener==='function')libraryField('library-search').addEventListener('input',renderLibraryCards);

initLibraryColorControls();
