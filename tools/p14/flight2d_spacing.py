"""Flight 2D: nearest-wingman distance per cell from a run folder.

Usage: python tools/p14/flight2d_spacing.py <run dir>
The run dir holds one Tacview .acmi and the run's missionReport logs.
Cells are told apart by activation time (AType:28); phases: transit before
the predicted WP 1, descent the next 180 s, then on station. Sampled once a
second; "<20 m" counts aircraft-seconds. Results: handoff/P14-probe-results.md,
"Flight 2D".
"""
import sys,re,glob,math,statistics as stt
run=sys.argv[1]
acmi=glob.glob(run+"/*.acmi")[0]
logs=sorted(glob.glob(run+"/missionReport*.txt"),key=lambda p:int(re.search(r"\[(\d+)\]",p).group(1)))
text="".join(open(p,encoding="utf-8",errors="replace").read() for p in logs).replace("\r","")
act={int(m.group(2)):int(m.group(1)) for m in re.finditer(r"T:(\d+) AType:28 OBJID:(\d+)",text)}
cells=["D0","D1","D2","DP"]; wp1={"D0":346,"D1":406,"D2":466,"DP":526}
starts=sorted(set(act.values())); cell_of={o:cells[starts.index(t)] for o,t in act.items()}
head=open(acmi,encoding="utf-8",errors="replace").read(400)
rt=re.search(r"ReferenceTime=\d+-\d+-\d+T(\d+):(\d+):(\d+)",head); off=(int(rt.group(1))-12)*3600+(int(rt.group(2))-30)*60+int(rt.group(3))
st={};t=0;last=-1
nn={c:{"transit":[],"descent":[],"station":[]} for c in cells}; close={c:0 for c in cells}; mins={c:1e9 for c in cells}
def snap():
  by={}
  for o,s in st.items():
    if "lat" in s and not s.get("gone"): by.setdefault(cell_of[o],[]).append(s)
  for c,ss in by.items():
    ph="transit" if t<wp1[c] else ("descent" if t<wp1[c]+180 else "station")
    for i,a in enumerate(ss):
      ds=[]
      for j,b in enumerate(ss):
        if i==j: continue
        dy=(a["lat"]-b["lat"])*111320; dx=(a["lon"]-b["lon"])*111320*math.cos(math.radians(a["lat"])); ds.append(math.hypot(dx,dy,a["alt"]-b["alt"]))
      if ds:
        d=min(ds); nn[c][ph].append(d); mins[c]=min(mins[c],d)
        if d<20: close[c]+=1
for line in open(acmi,encoding="utf-8",errors="replace"):
  line=line.rstrip("\n")
  if line.startswith("#"):
    t=float(line[1:])+off
    if int(t)!=last: last=int(t); snap()
    continue
  if line.startswith("-"):
    try:
      o=int(line[1:],16)
      if o in st: st[o]["gone"]=True
    except: pass
    continue
  m=re.match(r"([0-9a-f]+),T=([^,]*)",line)
  if not m: continue
  o=int(m.group(1),16)
  if o not in cell_of: continue
  p=m.group(2).split("|"); s=st.setdefault(o,{})
  for k,i in (("lon",0),("lat",1),("alt",2)):
    if len(p)>i and p[i]!="": s[k]=float(p[i])
def q(v,p): v=sorted(v); return v[int(p*(len(v)-1))] if v else float("nan")
for c in cells:
  r=" ".join("%s med %.0f p5 %.0f"%(ph,q(v,.5),q(v,.05)) for ph,v in nn[c].items())
  print(c,r,"| aircraft-seconds <20 m: %d, min %.0f m"%(close[c],mins[c]))
