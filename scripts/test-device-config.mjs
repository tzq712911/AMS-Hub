import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../core/static/config-app.js',import.meta.url),'utf8').replace(/^import .*\r?\n/,'').split('function bindForm(')[0];
function fixture(){
 const fields=new Map();const field=id=>{if(!fields.has(id))fields.set(id,{value:'',checked:false,addEventListener(){}});return fields.get(id);};
 const context=vm.createContext({document:{getElementById:field,querySelectorAll:()=>[]},window:{},derive_key(){},encrypt(){},decrypt(){},AbortSignal});
 vm.runInContext(source,context);
 for(const [id,value]of Object.entries({'printer-name':'H2D','printer-host':'192.0.2.20','printer-serial':'094123456789ABC','printer-access':'DEMO1234'}))field(id).value=value;
 return {context,field};
}
test('rejects the one-character serial that caused the startup panic, before submitting',()=>{
 const {context,field}=fixture();for(const serial of ['0','','H2D','0941234567/ABC','0941234567迁ABC','A'.repeat(65)]){
  field('printer-serial').value=serial;assert.throws(()=>context.collectPrinter(),/完整打印机 SN/);
 }
});
test('keeps full valid serial, custom device name and connection settings separate',()=>{
 const {context,field}=fixture();field('printer-name').value='工作室 H2D';const config=context.collectPrinter();
 assert.equal(config.serial,'094123456789ABC');assert.equal(config.name,'工作室 H2D');assert.equal(config.ip,'192.0.2.20');
});
test('rejects missing access code and duplicate device serial',()=>{
 const {context,field}=fixture();field('printer-access').value='';assert.throws(()=>context.collectPrinter(),/访问码/);
 field('printer-access').value='DEMO1234';vm.runInContext("state.bambu.printers=[{serial:'094123456789ABC',name:'Other'}]",context);
 assert.throws(()=>context.collectPrinter(),/序列号已配置/);
});


