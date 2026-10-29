"""Import identity fields from local Studio filament presets; never reads account config."""
import argparse, hashlib, json, re
from pathlib import Path

p=argparse.ArgumentParser()
p.add_argument('folder',type=Path,help='Studio user/.../filament folder')
p.add_argument('--output',type=Path,required=True)
p.add_argument('--printer',default='X2D')
a=p.parse_args()
root=Path(__file__).resolve().parents[1]
materials={line.split(',')[0] for line in (root/'core/data/materials.csv').read_text(encoding='utf-8').splitlines()[1:]}
docs={}
for f in a.folder.rglob('*.json'):
    try:
        d=json.loads(f.read_text(encoding='utf-8-sig'))
        if isinstance(d,dict) and d.get('name'):docs[d['name']]=(d,f)
    except (ValueError,OSError):continue
def merged(name,seen=()):
    if name in seen or name not in docs:return {}
    d,_=docs[name];return {**merged(d.get('inherits',''),seen+(name,)),**d}
def one(d,key,default=''):
    value=d.get(key,default);return value[0] if isinstance(value,list) and value else value
rows={};scores={};skipped=0
for name,(_,file) in docs.items():
    d=merged(name);code=d.get('filament_id','');material=one(d,'filament_type');brand=one(d,'filament_vendor')
    if not code or not brand or material not in materials:skipped+=1;continue
    low=str(one(d,'nozzle_temperature_range_low'));high=str(one(d,'nozzle_temperature_range_high'))
    if not low.isdigit() or not high.isdigit():skipped+=1;continue
    series=name.split(' @')[0].strip()
    if series.startswith(brand+' '):series=series[len(brand)+1:]
    identity=(brand,series,material)
    score=(100 if a.printer.casefold() in name.casefold() else 0)+(10 if '0.4' in name else 0)+(1 if file.parent==a.folder else 0)
    if identity in rows and scores[identity]>=score:continue
    setting=d.get('setting_id','')
    info=file.with_suffix('.info')
    if not setting and info.exists():
        match=re.search(r'^setting_id\s*=\s*(\S+)\s*$',info.read_text(encoding='utf-8-sig'),re.M)
        if match:setting=match[1]
    rows[identity]=dict(code='L'+hashlib.sha256('|'.join(identity).encode()).hexdigest()[:12],brand=brand,name=series,material=material,temp_min=int(low),temp_max=int(high),bambu_code=code,bambu_setting=setting,u1_vendor='Generic',u1_subtype='')
    scores[identity]=score
entries=sorted(rows.values(),key=lambda e:(e['brand'],e['material'],e['name']))
if len(entries)>256:raise SystemExit('More than 256 unique identities; narrow the source folder.')
a.output.parent.mkdir(parents=True,exist_ok=True)
a.output.write_text(json.dumps({'entries':entries},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
a.output.with_suffix('.glyphs.txt').write_text(''.join(e['brand']+e['name'] for e in entries),encoding='utf-8')
print(f'Imported {len(entries)} unique identities / {len({e["brand"] for e in entries})} brands; skipped {skipped} incomplete/unsupported profiles.')
print(a.output)
