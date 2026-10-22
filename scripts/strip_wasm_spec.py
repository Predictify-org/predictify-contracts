#!/usr/bin/env python3
"""Shrink a Soroban wasm: clear doc strings in the contractspecv0 section and
drop duplicate spec entries, so the contract fits on-chain size limits.
usage: stripspec.py in.wasm out.wasm"""
import sys,json,subprocess
def leb(d,i):
    r=0;s=0
    while True:
        b=d[i];i+=1;r|=(b&0x7f)<<s;s+=7
        if b<128:return r,i
def enc(n):
    o=bytearray()
    while True:
        b=n&0x7f;n>>=7
        if n:o.append(b|0x80)
        else:o.append(b);return bytes(o)
src,dst=sys.argv[1:3]
d=open(src,'rb').read()
i=8;secs=[]
while i<len(d):
    sid=d[i];s=i;i+=1;sz,i=leb(d,i);body=d[i:i+sz];i+=sz
    secs.append((sid,body))
out=bytearray(d[:8])
for sid,body in secs:
    if sid==0:
        n,j=leb(body,0);name=body[j:j+n].decode()
        if name=='contractspecv0':
            spec=body[j+n:]
            r=subprocess.run(['stellar','xdr','decode','--type','ScSpecEntry','--input','stream','--output','json'],input=spec,capture_output=True,check=True)
            ents=[json.loads(l) for l in r.stdout.decode().splitlines() if l.strip()]
            seen=set();keep=[]
            def strip(o):
                if isinstance(o,dict):
                    for k,v in list(o.items()):
                        if k=='doc': o[k]=''
                        else: strip(v)
                elif isinstance(o,list):
                    for v in o: strip(v)
            for e in ents:
                strip(e)
                k=json.dumps(e,sort_keys=True)
                if k in seen: continue
                seen.add(k);keep.append(e)
            # prune: keep functions + transitively referenced UDTs (drop events/unused types)
            def udts(o,acc):
                if isinstance(o,dict):
                    for k,v in o.items():
                        if k=='udt' and isinstance(v,dict) and 'name' in v: acc.add(v['name'])
                        udts(v,acc)
                elif isinstance(o,list):
                    for v in o: udts(v,acc)
            def ename(e):
                (k,v),=e.items(); return k,v.get('name')
            funcs=[e for e in keep if 'function_v0' in e]
            need=set();[udts(e,need) for e in funcs]
            bytype={}
            for e in keep:
                k,n=ename(e)
                if k!='function_v0' and k!='event_v0': bytype.setdefault(n,[]).append(e)
            done=set();todo=list(need)
            while todo:
                tn=todo.pop()
                if tn in done: continue
                done.add(tn)
                for e in bytype.get(tn,[]):
                    a=set();udts(e,a);todo+=list(a)
            keep=funcs+[e for e in keep if 'function_v0' not in e and 'event_v0' not in e and ename(e)[1] in done]
            inp='\n'.join(json.dumps(e) for e in keep).encode()
            # encode each separately & concatenate
            r=subprocess.run(['stellar','xdr','encode','--type','ScSpecEntry','--output','stream'],input=inp,capture_output=True,check=True)
            blob=r.stdout
            print('spec entries',len(ents),'->',len(keep),'bytes',len(spec),'->',len(blob),file=sys.stderr)
            body=enc(len(name))+name.encode()+blob
    out+=bytes([sid])+enc(len(body))+body
open(dst,'wb').write(out)
