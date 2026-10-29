import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../core/static/filament-library.js',import.meta.url),'utf8');
function fixture({badKey=false}={}) {
 const fields=new Map(),writes=[];
 const field=id=>{
  if(!fields.has(id))fields.set(id,{value:'',textContent:'',add(){},replaceChildren(){this.value='';}});
  return fields.get(id);
 };
 for(const [name,value] of Object.entries({brand:'Unknown brand',name:'PLA Plus',material:'PLA',color:'00CE9BFF',color_name:'绿色',temp_min:'190',temp_max:'240',bambu_code:'',bambu_setting:'',u1_vendor:'Generic',u1_subtype:''}))field('library-'+name).value=value;
 const existing={code:'Lold',brand:'Existing',name:'PETG',material:'PETG',bambu_code:'GFG99',u1_vendor:'Generic'};
 const context=vm.createContext({document:{getElementById:field},Option:class {},Date,Map,JSON,Error,Number,confirm:()=>true,
  getEncryptionKey:()=>'',decrypt:(_,text)=>text,
  fetch:async()=>({ok:true,text:async()=>badKey?'invalid encrypted text':JSON.stringify({library:{entries:[existing]}})}),
  sendData:async(_,body)=>{writes.push(body);return {ok:true,text:async()=>JSON.stringify({error_text:null})};}
 });
 vm.runInContext(source,context);return {context,writes,field};
}
test('saving a new identity preserves previously stored entries and keeps projections separate',async()=>{
 const {context,writes}=fixture();await context.savePrivateEntry();
 assert.equal(writes.length,1);assert.equal(writes[0].library.entries.length,2);
 assert.equal(writes[0].library.entries[0].code,'Lold');
 const added=writes[0].library.entries[1];assert.equal(added.brand,'Unknown brand');assert.equal(added.material,'PLA');assert.equal(added.bambu_code,'');
});
test('failed initial read never replaces the private library',async()=>{
 const {context,writes}=fixture({badKey:true});await context.savePrivateEntry();assert.equal(writes.length,0);
});
test('bulk import merges identities and preserves explicit printer projection',async()=>{
 const {context,writes}=fixture();
 await context.importPrivateLibrary({target:{value:'file',files:[{text:async()=>JSON.stringify({entries:[{code:'Lnew',brand:'真实品牌',name:'PLA',material:'PLA',bambu_code:'P1234567',u1_vendor:'Snapmaker',u1_subtype:'Speed PLA'}]})}]}});
 assert.equal(writes[0].library.entries.length,2);const added=writes[0].library.entries[1];assert.equal(added.brand,'真实品牌');assert.equal(added.bambu_code,'P1234567');assert.equal(added.u1_subtype,'Speed PLA');
});

test('RFID associations follow a saved preset or exact physical identity, never a different material',()=>{
 const {context}=fixture();const entries=[{code:'Lnew',brand:'真实品牌',name:'高速 PLA',material:'PLA'}];
 assert.equal(context.entryForBinding({preset_code:'Lnew',brand:'旧名称',name:'旧系列',material:'PLA'},entries).code,'Lnew');
 assert.equal(context.entryForBinding({preset_code:'GFL99',brand:'真实品牌',name:'高速 PLA',material:'PLA'},entries).code,'Lnew');
 assert.equal(context.entryForBinding({preset_code:'Lnew',brand:'真实品牌',name:'高速 PLA',material:'PETG'},entries),undefined);
 assert.equal(context.entryForBinding({preset_code:'GFL99',brand:'其他品牌',name:'高速 PLA',material:'PLA'},entries),undefined);
});

test('failed RFID refresh keeps the last successful read rather than claiming there are no tags',async()=>{
 const {context,field}=fixture();context.fetch=async()=>({ok:true,text:async()=>JSON.stringify({bindings:[{id:'1',tag_id:'04AABBCC',brand:'Existing',name:'PETG',material:'PETG',preset_code:'Lold',color:'123456FF'}],total:1,next_offset:null,error_text:null})});
 assert.equal(await context.loadRfidBindings(),true);
 context.fetch=async()=>({ok:true,text:async()=>JSON.stringify({bindings:[],error_text:'库存未就绪'})});
 assert.equal(await context.loadRfidBindings(),false);
 assert.equal(vm.runInContext('rfidBindings.length',context),1);
 assert.match(field('rfid-status').textContent,/保留上次关联/);
});


test('colors are normalized and are required for new or edited entries',async()=>{
 const {context,writes,field}=fixture();assert.equal(context.normalizeLibraryColor('#a1B2c3'),'A1B2C3FF');assert.equal(context.normalizeLibraryColor('12345678'),'12345678');assert.equal(context.normalizeLibraryColor('blue'),'');
 field('library-color').value='';await context.savePrivateEntry();assert.equal(writes.length,0);assert.match(field('library-status').textContent,/颜色/);
 field('library-color').value='#00ce9b';await context.savePrivateEntry();assert.equal(writes[0].library.entries[1].color,'00CE9BFF');assert.equal(writes[0].library.entries[1].color_name,'绿色');
});
test('color variants associate with matching RFID colors, with legacy colorless fallback',()=>{
 const {context}=fixture();const entries=[{code:'Lred',brand:'Local',name:'PLA Plus',material:'PLA',color:'FF0000FF'},{code:'Lblue',brand:'Local',name:'PLA Plus',material:'PLA',color:'0000FFFF'}];
 const binding={preset_code:'old',brand:'Local',name:'PLA Plus',material:'PLA',color:'0000ffff'};assert.equal(context.entryForBinding(binding,entries).code,'Lblue');
 binding.color='00FF00FF';assert.equal(context.entryForBinding(binding,entries),undefined);entries.push({code:'Lold',brand:'Local',name:'PLA Plus',material:'PLA'});assert.equal(context.entryForBinding(binding,entries).code,'Lold');
});
test('brand and material classifications combine with RFID and color searches',()=>{
 const {context}=fixture();const entries=[{code:'La',brand:'A',name:'PLA',material:'PLA',color:'FF0000FF',color_name:'红色'},{code:'Lb',brand:'A',name:'PETG',material:'PETG'},{code:'Lc',brand:'B',name:'PLA',material:'PLA'}];const bindings=[{preset_code:'La',material:'PLA',tag_id:'04AABBCC'}];
 assert.equal(context.visibleLibraryEntries(entries,bindings,{groupBy:'brand',category:'A'}).length,2);assert.equal(context.visibleLibraryEntries(entries,bindings,{groupBy:'material',category:'PLA'}).length,2);
 assert.equal(context.visibleLibraryEntries(entries,bindings,{groupBy:'brand',category:'A',search:'04aabb'}).length,1);assert.equal(context.visibleLibraryEntries(entries,bindings,{search:'#ff0000'}).length,1);
});


test('same material family groups all color variants and RFID side by side without merging brands',()=>{
 const {context}=fixture();const red={code:'Lred',brand:'Local',name:'PLA Plus',material:'PLA',color:'FF0000FF'};
 const blue={...red,code:'Lblue',color:'0000FFFF'},other={...red,code:'Lother',brand:'Other'};
 const entries=[red,other,blue];const families=context.materialFamilies(entries);assert.equal(families.length,2);assert.equal(families[0].length,2);
 const bindings=[{tag_id:'A',preset_code:'Lred',material:'PLA'},{tag_id:'B',preset_code:'Lblue',material:'PLA'},{tag_id:'C',preset_code:'Lred',material:'PLA'},{tag_id:'D',preset_code:'Lother',material:'PLA'}];
 assert.deepEqual(Array.from(context.familyBindings(families[0],bindings,entries),b=>b.tag_id),['A','B','C']);
});


test('web brand search includes full catalog plus private brands without duplicate Bambu aliases',()=>{
 const {context}=fixture();const rows=context.brandSuggestions([{brand:'私有品牌'}],['Bambu','Bambu Lab','Rare filament','eSUN']);
 assert.ok(rows.includes('私有品牌'));assert.ok(rows.includes('Rare filament'));assert.equal(rows.filter(x=>x==='Bambu Lab').length,1);
});
