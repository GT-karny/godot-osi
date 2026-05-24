"""Extract OpenDRIVE traffic-signal/light icons from the ASAM OpenDRIVE
Signal_Base_catalog PDF into addons/godot_osi/icons/signals/odr_<type>_<subtype>.png.

Renders each catalogue cell with alpha=True so round signs keep transparent
corners (the soft-mask is applied by the PDF renderer; raw embedded extraction
would leave opaque backgrounds). Round signs carry a soft-mask so their corners
are already transparent; rectangular housings (traffic-light boxes) instead have
an OPAQUE white background baked into the artwork, so we additionally flood-fill
near-white from the image borders to transparency (interior white is preserved).
Requires PyMuPDF (pip install pymupdf).

Run from the repo root:  python tools/extract_signal_icons.py
After re-extracting, regenerate the classification table: python tools/gen_signal_catalog.py
"""
import fitz, os, re
from collections import deque

def strip_white_border(pm, thr=235):
    """Make border-connected near-white pixels transparent (background removal)."""
    if not pm.alpha:
        pm = fitz.Pixmap(pm, 1)
    w, h, n = pm.width, pm.height, pm.n  # n == 4 (RGBA)
    buf = bytearray(pm.samples)
    def i(x, y): return (y * w + x) * n
    def white(p): return buf[p] >= thr and buf[p+1] >= thr and buf[p+2] >= thr and buf[p+3] > 0
    dq = deque()
    for x in range(w):
        dq.append((x, 0)); dq.append((x, h - 1))
    for y in range(h):
        dq.append((0, y)); dq.append((w - 1, y))
    seen = bytearray(w * h)
    while dq:
        x, y = dq.popleft()
        if x < 0 or y < 0 or x >= w or y >= h or seen[y * w + x]:
            continue
        seen[y * w + x] = 1
        p = i(x, y)
        if white(p):
            buf[p + 3] = 0
            dq.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)])
    return fitz.Pixmap(fitz.csRGB, w, h, bytes(buf), True)

def trim(pm):
    """Crop away fully-transparent margins, tightening the icon to its content."""
    if not pm.alpha:
        return pm
    w, h, n = pm.width, pm.height, pm.n
    src = pm.samples
    minx, miny, maxx, maxy = w, h, -1, -1
    for y in range(h):
        base = y * w * n
        for x in range(w):
            if src[base + x * n + 3] > 0:
                if x < minx: minx = x
                if x > maxx: maxx = x
                if y < miny: miny = y
                if y > maxy: maxy = y
    if maxx < 0:
        return pm
    nw, nh = maxx - minx + 1, maxy - miny + 1
    out = bytearray(nw * nh * n)
    for yy in range(nh):
        s = ((miny + yy) * w + minx) * n
        d = yy * nw * n
        out[d:d + nw * n] = src[s:s + nw * n]
    return fitz.Pixmap(pm.colorspace, nw, nh, bytes(out), True)
doc = fitz.open("temp/opendrive/additional_content/Signal_Base_catalog.pdf")
OUT = "godot/addons/godot_osi/icons/signals"; os.makedirs(OUT, exist_ok=True)
TYPE_X=[123,280,438]; SUB_X=[181,337,493]; TOL=18
MAT=fitz.Matrix(5,5)
def near(x,a):
    for i,v in enumerate(a):
        if abs(x-v)<=TOL: return i
    return None
def save(page,r,path,clean=True):
    # alpha=True keeps the page background transparent so masked/round signs
    # do not get composited onto white (the earlier white-margin artifact).
    # Inset slightly so neighbouring table grid-lines are not captured at the
    # cell edges (they survive white-removal and would block a tight trim).
    r=fitz.Rect(r.x0+1.0, r.y0+1.0, r.x1-1.0, r.y1-1.0) if clean else r
    pm=page.get_pixmap(clip=r, matrix=MAT, alpha=True)
    if clean:
        pm=strip_white_border(pm)  # remove opaque white background of box signs
        pm=trim(pm)                # crop the now-transparent margins tight
    pm.save(path)
n=0
for pno in [4,5,6,7]:
    page=doc[pno]
    imgs=[]
    for img in page.get_images(full=True):
        for r in page.get_image_rects(img[0]):
            if r.y0<80 or r.width>60: continue
            imgs.append((img[0],r))
    words=[w for w in page.get_text("words") if w[4].strip()]
    rows={}
    for w in words:
        x0,y0,x1,y1,txt=w[0],w[1],w[2],w[3],w[4].strip()
        gt=near(x0,TYPE_X); gs=near(x0,SUB_X); yc=round((y0+y1)/2)
        if gt is not None and (re.match(r'^1\.000\.\d+$',txt) or re.match(r'^[FWA]$',txt)):
            rows.setdefault((gt,yc),{})['type']=txt; rows[(gt,yc)]['ty']=(y0+y1)/2
        elif gs is not None and (txt=='-' or re.match(r'^\d+[A-Z]?$',txt) or txt=='X'):
            for (g,yy),d in rows.items():
                if g==gs and abs(yy-yc)<=6 and 'sub' not in d: d['sub']=txt
    for (g,yc),d in rows.items():
        if 'type' not in d: continue
        ty=d['ty']; type_x=TYPE_X[g]; best=None; bd=1e9
        for xref,r in imgs:
            if r.x1>type_x or (type_x-r.x1)>130: continue
            ic=(r.y0+r.y1)/2; dd=abs(ic-ty)
            if dd<bd: bd=dd; best=(xref,r)
        if best is None or bd>40: continue
        xref,r=best; sub=d.get('sub','-')
        t=d['type'].replace('.',''); s='none' if sub in ('-','') else sub
        save(page,r, os.path.join(OUT,f"odr_{t}_{s}.png")); n+=1
# road-marking photos (opaque; alpha harmless)
page=doc[5]
photos=sorted([(img[0],r) for img in page.get_images(full=True) for r in page.get_image_rects(img[0]) if r.width>60 and r.y0>350], key=lambda x:x[1].y0)
for (xref,r),name in zip(photos[:2],["odr_1000003_none.png","odr_1000004_none.png"]):
    save(page,r, os.path.join(OUT,name), clean=False); n+=1  # real photos: keep as-is
print("extracted",n,"icons (alpha preserved, white background removed)")
