#!/usr/bin/env python3
"""Generate the OpenDRIVE signal classification table (TSV) from esmini's
authoritative `traffic_light_type_map` plus the OpenDRIVE Signal_Base_catalog.

Source (read-only upstream):
  external/esmini/.../RoadManager/RoadManager.hpp  -> traffic_light_type_map
Output:
  crates/godot-osi/src/road/signal_catalog.tsv

Columns: type<TAB>subtype<TAB>category<TAB>subcategory<TAB>color<TAB>arrow<TAB>
         osi_type_name<TAB>nr_lamps<TAB>icon<TAB>label_en<TAB>label_ja
Run:  temp/pdfvenv/Scripts/python.exe tools/gen_signal_catalog.py
"""
import re, os

HPP = "external/esmini/EnvironmentSimulator/Modules/RoadManager/RoadManager.hpp"
OUT = "crates/godot-osi/src/road/signal_catalog.tsv"

src = open(HPP, encoding="utf-8", errors="replace").read()
# isolate the map body
m = re.search(r"traffic_light_type_map\s*=\s*\{(.*?)\}\};", src, re.S)
body = m.group(1)

# Each entry: {"KEY", {TYPE_x, N, {COLOR..}, {ICON..}}}
entry_re = re.compile(
    r'\{"([^"]+)"\s*,\s*\{\s*(\w+)\s*,\s*(\d+)\s*,\s*\{([^}]*)\}\s*,\s*\{([^}]*)\}\s*\}\}',
    re.S)

def icon_to_arrow(icon):
    a = icon.replace("ICON_ARROW_", "")
    table = {
        "LEFT":"left","RIGHT":"right","STRAIGHT_AHEAD":"straight",
        "STRAIGHT_AHEAD_LEFT":"straight_left","STRAIGHT_AHEAD_RIGHT":"straight_right",
        "DIAG_LEFT":"diag_left","DIAG_RIGHT":"diag_right",
        "DOWN":"down","DOWN_LEFT":"down_left","DOWN_RIGHT":"down_right",
        "LEFT_RIGHT":"left_right","CROSS":"cross",
    }
    return table.get(a, "none")

ARROW_JA = {"left":"左","right":"右","straight":"直進","straight_left":"直進+左",
    "straight_right":"直進+右","diag_left":"左斜め","diag_right":"右斜め","down":"下",
    "down_left":"左下","down_right":"右下","left_right":"左右","cross":"×(進入禁止)","none":""}
COLOR_JA = {"red":"赤","yellow":"黄","green":"青","multi":""}

def classify(icons, colors):
    """subcategory from icon set"""
    iset = set(icons)
    if iset & {"ICON_PEDESTRIAN","ICON_WALK","ICON_DONT_WALK"}:
        return "pedestrian"
    if iset == {"ICON_BICYCLE"} or "ICON_BICYCLE" in iset and not any("ARROW" in i for i in iset):
        return "bicycle"
    if "ICON_PEDESTRIAN_AND_BICYCLE" in iset:
        return "pedestrian_bicycle"
    if iset & {"ICON_TRAM"}:
        return "tram"
    if iset & {"ICON_BUS","ICON_BUS_AND_TRAM"}:
        return "bus"
    if any("ARROW" in i for i in iset):
        return "vehicle_arrow"
    return "vehicle"

rows = []
for mt in entry_re.finditer(body):
    key, tl_type, n, colors_s, icons_s = mt.groups()
    n = int(n)
    colors = [c.strip().replace("LampColor::","").replace("COLOR_","").lower()
              for c in colors_s.split(",") if c.strip()]
    icons = [i.strip().replace("LampIcon::","") for i in icons_s.split(",") if i.strip()]
    if "." in key:
        typ, sub = key.split(".", 1)
    else:
        typ, sub = key, "none"
    subcat = classify(icons, colors)
    arrow = icon_to_arrow(icons[0]) if icons else "none"
    # color: multi-lamp head -> "multi"; single lamp -> that lamp's lit color
    color = "multi" if n > 1 else (colors[0] if colors else "none")
    icon_file = f"odr_{typ}_{sub}"
    # English label
    head = {"vehicle":"Vehicle traffic light","vehicle_arrow":"Vehicle arrow light",
            "pedestrian":"Pedestrian light","bicycle":"Bicycle light",
            "pedestrian_bicycle":"Pedestrian/bicycle light","tram":"Tram signal",
            "bus":"Bus signal"}[subcat]
    head_ja = {"vehicle":"車両用信号機","vehicle_arrow":"車両用矢印信号",
            "pedestrian":"歩行者用信号","bicycle":"自転車用信号",
            "pedestrian_bicycle":"歩行者・自転車用信号","tram":"路面電車用信号",
            "bus":"バス用信号"}[subcat]
    extras=[]; extras_ja=[]
    if arrow!="none":
        extras.append(arrow.replace("_"," ")); extras_ja.append(ARROW_JA[arrow])
    if n>1:
        extras.append(f"{n}-aspect"); extras_ja.append(f"{n}灯")
    else:
        extras.append(color); extras_ja.append(COLOR_JA.get(color,color))
    label_en = head + (" (" + ", ".join(x for x in extras if x) + ")" if any(extras) else "")
    label_ja = head_ja + ("(" + "・".join(x for x in extras_ja if x) + ")" if any(extras_ja) else "")
    rows.append([typ,sub,"traffic_light",subcat,color,arrow,"TYPE_UNKNOWN",str(n),icon_file,label_en,label_ja])

# Catalog-only entries not present in esmini's traffic_light_type_map.
# Sourced from OpenDRIVE Signal_Base_catalog (icons extracted in icons/signals/).
MANUAL_ROWS = [
    # Road markings (crossings)
    ["1000003","none","road_marking","pedestrian_crossing","none","none","TYPE_ZEBRA_CROSSING","0","odr_1000003_none","Pedestrian crossing (road marking)","横断歩道(路面標示)"],
    ["1000004","none","road_marking","bicycle_crossing","none","none","TYPE_UNKNOWN","0","odr_1000004_none","Bicycle crossing (road marking)","自転車横断帯(路面標示)"],
    # Single amber/special lights (catalog table 5, not in lamp map)
    ["1000014","none","traffic_light","vehicle","yellow","none","TYPE_UNKNOWN","1","odr_1000014_none","Amber flashing light","黄点滅信号"],
    ["1000016","none","traffic_light","pedestrian_bicycle","yellow","none","TYPE_UNKNOWN","1","odr_1000016_none","Pedestrian/bicycle warning light","歩行者・自転車注意灯"],
    ["1000017","none","traffic_light","bus","yellow","none","TYPE_UNKNOWN","1","odr_1000017_none","Tram/bus signal","路面電車・バス用信号"],
    ["1000018","none","traffic_light","other","yellow","none","TYPE_UNKNOWN","1","odr_1000018_none","Equestrian warning light","騎馬注意灯"],
    ["1000019","none","traffic_light","pedestrian_bicycle","yellow","none","TYPE_UNKNOWN","1","odr_1000019_none","Pedestrian/bicycle warning light","歩行者・自転車注意灯"],
]
# Tram signals (catalog table 7): type letters F / W / A.
TRAM = {
    "F": [("0","stop bar","停止(横棒)"),("1","caution bar","注意(縦棒)"),("2","proceed diagonal","進行(斜め)"),
          ("3","stop diagonal","停止(斜め)"),("4","dot","点"),("5","down triangle","下三角")],
    "W": [("0","cross (stop)","×(停止)"),("1","up","上"),("2","right","右"),("3","left","左"),
          ("11","up outline","上(白枠)"),("12","right outline","右(白枠)"),("13","left outline","左(白枠)"),("14","down outline","下(白枠)")],
    "A": [("1","ring","環状"),("X","cross","×"),("2B","variant","派生")],
}
for letter, subs in TRAM.items():
    for sub, en, ja in subs:
        MANUAL_ROWS.append([letter, sub, "tram_signal", "tram", "none", "none", "TYPE_UNKNOWN", "0",
                            f"odr_{letter}_{sub}", f"Tram signal {letter} ({en})", f"路面電車信号 {letter}({ja})"])
rows.extend(MANUAL_ROWS)

# sort by type then numeric subtype
def skey(r):
    s=r[1]
    return (r[0], -1 if s=="none" else int(re.sub(r"\D","",s) or 0), s)
rows.sort(key=skey)

# Blank the icon field for entries whose catalog art was not extracted
# (esmini-only extensions absent from the OpenDRIVE Signal_Base_catalog).
ICON_DIR = "godot/addons/godot_osi/icons/signals"
for r in rows:
    if r[8] and not os.path.exists(os.path.join(ICON_DIR, r[8] + ".png")):
        r[8] = ""

os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(OUT,"w",encoding="utf-8",newline="\n") as f:
    f.write("# Generated by tools/gen_signal_catalog.py from esmini traffic_light_type_map + OpenDRIVE Signal_Base_catalog. Do not edit by hand.\n")
    f.write("type\tsubtype\tcategory\tsubcategory\tcolor\tarrow\tosi_type_name\tnr_lamps\ticon\tlabel_en\tlabel_ja\n")
    for r in rows:
        f.write("\t".join(r)+"\n")
print(f"wrote {len(rows)} rows -> {OUT}")
