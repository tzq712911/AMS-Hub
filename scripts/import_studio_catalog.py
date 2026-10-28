"""Extract filament identity facts from an installed Bambu Studio resource tree.

Does not read user accounts, access codes or user project files.
"""
import argparse,csv,json
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument("resources",type=Path);a=p.parse_args()
root=Path(__file__).resolve().parents[1]
folder=a.resources/'profiles/BBL/filament'
docs={}
for f in folder.glob('*.json'):
    d=json.loads(f.read_text(encoding='utf-8-sig'))
    if isinstance(d,dict) and 'name' in d:docs[d['name']]=d
def merged(name,seen=()):
    if name in seen or name not in docs:return {}
    d=docs[name];return {**merged(d.get('inherits',''),seen+(name,)),**d}
def one(d,k,default=''):
    v=d.get(k,default);return v[0] if isinstance(v,list) and v else v
old={r[0]:r for r in csv.reader((root/'core/data/base-filaments-index.csv').open(encoding='utf-8-sig')) if len(r)>=5}
rows={}
for name,d in docs.items():
    if '@base' not in name or not d.get('filament_id'):continue
    m=merged(name);code=d['filament_id'];display=name.split(' @')[0]
    vendor=one(m,'filament_vendor','Generic');material=one(m,'filament_type')
    if not material:continue
    previous=old.get(code,[])
    lo=one(m,'nozzle_temperature_range_low',previous[2] if len(previous)>3 else '')
    hi=one(m,'nozzle_temperature_range_high',previous[3] if len(previous)>3 else '')
    if not str(lo).isdigit() or not str(hi).isdigit():continue
    rows[code]=[code,display,lo,hi,material,vendor]
for code,r in old.items():
    if code not in rows:rows[code]=r[:5]+[r[5] if len(r)>5 else ('Bambu Lab' if r[1].startswith('Bambu ') else 'Generic')]
with (root/'core/data/base-filaments-index.csv').open('w',encoding='utf-8',newline='') as f:csv.writer(f,lineterminator='\n').writerows(sorted(rows.values()))
colors=json.loads((folder/'filaments_color_codes.json').read_text(encoding='utf-8-sig'))['data']
with (root/'core/data/bambu-color-names.csv').open('w',encoding='utf-8',newline='') as f:
    w=csv.writer(f,lineterminator='\n')
    for c in colors:
        if len(c.get('fila_color',[]))!=1:continue  # multicolour must not be misrepresented as a solid colour
        rgba=c['fila_color'][0].lstrip('#').upper()
        if len(rgba)==6:rgba+='FF'
        if len(rgba)==8:w.writerow([c['fila_id'],rgba,c['fila_color_name'].get('en',''),c.get('fila_color_code','')])
print('Catalog:',len(rows),'presets;',len(set(r[5] for r in rows.values())),'brands;',len(colors),'source colour records')
