"""Source-image selections. All rectangles use global image coordinates and exclusive ends."""

AREAS = [
    ("01-window-frame", "Window frame", (28, 22, 1620, 882)),
    ("02-title-bar", "Title bar", (29, 23, 1618, 54)),
    ("03-left-rail", "Left rail", (29, 77, 76, 784)),
    ("04-explorer", "Explorer", (106, 77, 281, 455)),
    ("05-source-control", "Source Control", (106, 531, 281, 330)),
    ("06-editor", "Editor", (387, 77, 832, 549)),
    ("07-bottom-panel", "Bottom panel", (387, 626, 832, 235)),
    ("08-right-dock", "Right dock", (1219, 77, 428, 784)),
    ("09-status-bar", "Status bar", (29, 862, 1618, 41)),
]

# Rectangles were located in the source, then local ridge profiles are recorded by measure.py.
# Radius is a contour-fit search interval, not a claim about the original component token.
RECTS = []
TEXT = []
ICONS = []

def rect(area, id, bounds, *, radius=(0, 0), border=True, fill=True, padding="not observable", note="", sample=None, annotate=True):
    RECTS.append(dict(area=area, id=id, bounds=list(bounds), radius_search=list(radius), border=border, fill=fill, padding=padding, note=note, sample=sample, annotate=annotate))

def text(area, id, content, roi, style="ui", note="", annotate=False):
    TEXT.append(dict(area=area, id=id, content=content, roi=list(roi), style=style, note=note, annotate=annotate))

def icon(area, id, roi, *, color="neutral", note="", annotate=False):
    ICONS.append(dict(area=area,id=id,roi=list(roi),color=color,note=note,annotate=annotate))

# 1. Frame.
rect(1,"window",(28,22,1620,882),radius=(8,20),sample=(800,800),padding="left 0; top title band 55; bottom status band 42",note="Visible frame envelope; AA contour extends into adjacent pixels. Screenshot includes desktop outside this rectangle.")

# 2. Title bar.
rect(2,"title-band",(29,23,1618,54),border=False,sample=(1050,50),padding="brand x+19; controls centered near y=49",note="Bottom divider ridge at y=76.")
rect(2,"command-palette",(496,31,503,36),radius=(3,9),sample=(750,40),padding="icon left 11; label left 37; y 10",note="Visible key hint uses Ctrl+Shift+P, not a separate keycap.")
rect(2,"room-pill",(1085,31,212,36),radius=(3,9),sample=(1200,35),padding="icon left 13; label left 46")
icon(2,"brand-logo",(46,34,31,30),color="multi",annotate=True)
text(2,"brand-label","Forge Room",(88,38,111,25),"brand",annotate=True)
icon(2,"back",(373,39,21,20),annotate=True)
icon(2,"forward",(413,39,21,20),note="Visibly dimmer than Back.")
icon(2,"palette-search",(505,37,22,23))
text(2,"palette-label","Command Palette  (Ctrl+Shift+P)",(531,39,219,21),"ui")
icon(2,"title-room-people",(1097,38,27,22),color="multi",note="Small green presence mark at lower right.")
text(2,"room-label","Room: Product Launch",(1130,39,155,21),"ui")
icon(2,"title-run",(1346,38,19,21),annotate=True)
icon(2,"title-chat",(1393,37,22,22),annotate=True)
icon(2,"title-bell",(1443,37,23,25))
icon(2,"notification-dot",(1455,33,15,15),color="red",annotate=True)
icon(2,"window-minimize",(1513,44,19,8),annotate=True)
icon(2,"window-maximize",(1560,39,19,19),annotate=True)
icon(2,"window-close",(1608,38,21,21),annotate=True)

# 3. Rail.
rect(3,"rail-band",(29,77,76,784),border=False,sample=(40,300),padding="icons centered approximately x=66; first center y=113")
rect(3,"room-active-tile",(39,439,56,46),radius=(5,12),sample=(86,465),padding="room ink approximately x+15,y+14",note="No separate count badge is visible in the rail.")
for name, roi in [
    ("explorer",(53,101,25,23)),("search",(53,149,25,25)),
    ("source-control",(55,198,23,25)),("run-debug",(56,249,23,24)),
    ("extensions",(53,299,27,27)),("ai",(53,348,27,26)),
    ("terminal",(54,397,24,23)),("room",(51,448,31,27)),
]: icon(3,"rail-"+name,roi,annotate=True)

# 4. Explorer.
rect(4,"explorer-header",(106,77,281,49),border=False,sample=(270,112),padding="label x+19; action icons at x=296 and x=342")
text(4,"explorer-heading","Explorer",(123,91,75,24),"panel-heading",annotate=True)
icon(4,"new-file",(294,91,22,23),annotate=True)
icon(4,"explorer-refresh",(339,91,23,24),annotate=True)
rect(4,"selected-tree-row",(183,253,191,26),radius=(2,7),border=False,sample=(350,264),padding="chevron x+25; folder x+43; label x+68",note="Only Room is selected; no visible row outline.")
TREE = [
    ("forge-room-app",0,142,144,None,"down"),
    ("apps",1,168,176,"folder","down"),
    ("desktop",2,193,194,"folder","down"),
    ("src",3,218,211,"folder","down"),
    ("features",4,242,232,"folder","down"),
    ("room",5,266,251,"folder","right"),
    ("editor",5,291,251,"folder","right"),
    ("ai",5,315,251,"folder","right"),
    ("scm",5,339,251,"folder","right"),
    ("components",4,363,232,"folder","down"),
    ("services",4,388,236,"file",None),
    ("main.ts",4,412,236,"file",None),
    ("assets",2,436,217,"folder","right"),
    ("package.json",1,459,196,"json",None),
    ("tsconfig.json",1,483,196,"ts",None),
    ("README.md",1,507,196,"readme",None),
]
for name,level,cy,tx,kind,chev in TREE:
    slug=name.replace(".","-")
    text(4,"tree-"+slug,name,(tx,cy-10, max(25, min(175,len(name)*10)),21),"tree",note=f"Visible center y={cy}; depth label={level}.")
    if kind:
        ix=tx-25
        icon(4,"tree-icon-"+slug,(ix,cy-9,20,19),note=f"{kind} marker; source outline traced.")
    if chev:
        cx={0:127,1:138,2:155,3:175,4:194,5:212}[level] if name!="assets" else 178
        icon(4,"tree-chevron-"+slug,(cx-6,cy-6,12,13),note=chev)

# 5. SCM.
rect(5,"scm-panel",(106,531,281,330),radius=(6,15),sample=(120,710),padding="content x+21; right 20")
rect(5,"scm-header",(106,531,281,44),border=False,fill=False,padding="heading x+22,y+17; divider y=574")
text(5,"scm-heading","Source Control",(127,544,128,24),"section-heading",annotate=True)
icon(5,"scm-more",(348,546,20,14),annotate=True)
text(5,"changes-count","Changes (3)",(127,585,100,24),"tab",annotate=True)
for i,(name,label,cy,color) in enumerate([
    ("room","src/features/room/Room.tsx",628,"blue"),
    ("types","src/features/room/types.ts",658,"blue"),
    ("agents","src/lib/agents.ts",690,"red"),
]):
    rect(5,f"badge-{name}",(129,cy-11,31,23),radius=(2,6),border=False,sample=(132,cy-7),padding="glyph centered",note="M = modified" if i<2 else "A = added")
    text(5,f"badge-letter-{name}","M" if i<2 else "A",(137,cy-8,16,18),"badge-"+color)
    text(5,f"change-path-{name}",label,(170,cy-10,184,22),"small-ui")
rect(5,"commit-button",(127,715,240,33),radius=(3,9),sample=(180,731),padding="label centered; dropdown starts x=326",note="Main segment 199 px; dropdown segment 41 px.")
rect(5,"commit-dropdown-segment",(326,716,40,31),border=False,fill=False,padding="chevron centered near (347,732)")
text(5,"commit-label","Commit...",(206,722,71,22),"ui")
icon(5,"commit-chevron",(340,726,15,13))
rect(5,"sync-button",(127,757,240,35),radius=(3,9),sample=(140,780),padding="icon x+91; label x+119")
icon(5,"sync-icon",(217,763,22,23))
text(5,"sync-label","Sync",(246,764,38,22),"ui")
rect(5,"open-pr-button",(127,801,240,36),radius=(3,9),sample=(145,809),padding="text x+45; external-link right 10")
text(5,"open-pr-label","Open Pull Request...",(171,810,141,22),"ui")
icon(5,"open-pr-link",(339,809,20,20))

# 6. Editor.
rect(6,"editor-tab-strip",(387,77,832,49),border=False,sample=(1150,99),padding="active label x+45; plus near x=984")
rect(6,"room-tab",(388,80,158,46),radius=(5,13),border=False,sample=(470,86),padding="label x+44; close right 14",note="Visible active underline at y=122..125.")
rect(6,"room-tab-underline",(391,122,151,3),border=False,sample=(470,123),padding="0",note="Bright core plus a wider red fringe; source blur parameters unrecoverable.")
text(6,"room-tab-label","Room.tsx",(431,92,72,22),"tab",annotate=True)
icon(6,"room-tab-close",(518,94,16,16))
text(6,"agent-tab-label","AgentSession.ts",(585,92,118,22),"tab")
text(6,"schema-tab-label","room.schema.json",(789,92,136,22),"tab")
icon(6,"editor-new-tab",(973,90,22,24),annotate=True)
rect(6,"breadcrumb-band",(388,127,829,31),border=False,fill=False,padding="first label x+20; baseline near y=146")
for id,content,x,w in [("apps","apps",408,33),("desktop","desktop",457,57),("src","src",531,24),("features","features",566,59),("room","room",639,39),("file","Room.tsx",718,69)]:
    text(6,"breadcrumb-"+id,content,(x,132,w,22),"breadcrumb")
for i,x in enumerate([444,515,559,630,683]): icon(6,f"breadcrumb-chevron-{i}",(x,135,10,14))
icon(6,"breadcrumb-ts",(696,134,18,17))
rect(6,"line-number-gutter",(388,159,67,453),border=False,fill=False,padding="line-number right edge near x=440; code starts x=474",note="First visible text ink starts around x=474; 19 px gap from gutter end.")
rect(6,"active-line",(420,428,797,20),border=False,sample=(470,438),padding="0",note="Warm strip across row 14; line number red. Caret near x=1041.")
rect(6,"caret",(1040,430,2,18),border=False,sample=(1041,438),padding="0",note="Observed amber vertical stroke; blink timing cannot be measured from a still image.")
CODE = [
    ("1",474,177,"import { useRoom, useAgents } from '@forge-room/sdk'",1150),
    ("2",474,198,"import { RoomHeader, MemberList, ThreadList, TaskList } from '../components'",1150),
    ("3",474,218,"import { StartAgentSession } from './actions'",1000),
    ("4",474,238,"",474),
    ("5",474,258,"export function Room() {",850),
    ("6",495,278,"const { room, members, threads, tasks } = useRoom()",1050),
    ("7",495,298,"const { agents, startSession } = useAgents(room.id)",1090),
    ("8",474,318,"",474),
    ("9",495,338,"return (",680),
    ("19",516,358,'<div className="room">',780),
    ("11",537,378,"<RoomHeader",690),
    ("12",557,398,"room={room}",710),
    ("13",557,418,"onInvite={() => openInvite()}",880),
    ("14",557,438,"onStartAgent={() => startSession({ agent: 'builder' })}",1043),
    ("15",537,458,"/>",585),
    ("16",537,478,'<div className="grid grid-cols-2 gap-6">',894),
    ("17",557,498,"<MemberList members={members} />",894),
    ("18",557,518,"<TaskList tasks={tasks} />",894),
    ("19",537,538,"</div>",650),
    ("20",516,558,"</div>",650),
    ("21",495,578,")",550),
    ("22",474,598,"}",500),
]
for index,(number,x,cy,content,rx) in enumerate(CODE):
    text(6,f"line-number-{index+1:02}",number,(421,cy-9,23,20),"line-number-active" if number=="14" else "line-number")
    if content:
        text(6,f"code-line-{index+1:02}",content,(x,cy-9,rx-x,20),"code",note="Literal screenshot transcription; the source shows '19' after '9'.")
rect(6,"floating-toolbar",(740,363,444,37),radius=(5,12),sample=(1176,369),padding="left inset 6; top inset 4; button gaps 10,11,9,11",note="No editor reflow; overlay above the active line.")
for id,b,label,tx in [
    ("explain",(746,367,91,28),"Explain",779),
    ("refactor",(847,367,96,28),"Refactor",879),
    ("add-tests",(954,367,104,28),"Add tests",984),
    ("fix",(1067,367,62,28),"Fix",1101),
    ("more",(1140,367,39,28),None,None),
]:
    rect(6,"toolbar-"+id,b,radius=(3,8),sample=(b[0]+b[2]-7,b[1]+5),padding="icon left 10; label after 22 px",annotate=False)
    icon(6,"toolbar-icon-"+id,(b[0]+10,b[1]+5,20,19),color="red" if id in ("explain","fix") else "neutral")
    if label: text(6,"toolbar-label-"+id,label,(tx,371,b[0]+b[2]-tx-3,22),"ui")
rect(6,"agent-dropdown",(894,451,307,145),radius=(5,13),sample=(1189,586),padding="row inset 6; rows approximately 27 px; label x+43",note="Small pointer points toward (915,447); popup occludes the code.")
rect(6,"agent-selected-row",(900,456,295,27),radius=(3,9),sample=(1100,459),padding="cube x+9; title x+36; role right 10")
for name,role,cy in [("builder","Agent (recommended)",470),("reviewer","Agent",498),("tester","Agent",524),("docs","Agent",551),("custom...","Create custom agent...",578)]:
    slug=name.replace("...","")
    icon(6,"agent-cube-"+slug,(908,cy-10,21,21))
    text(6,"agent-name-"+slug,name,(936,cy-10,112,23),"tab")
    tx=1057 if name=="builder" else (1051 if name=="custom..." else 1148)
    text(6,"agent-role-"+slug,role,(tx,cy-10,1198-tx-8,23),"small-helper")

# 7. Bottom panel.
rect(7,"bottom-tabs",(388,626,829,44),border=False,sample=(937,646),padding="first tab x+15; y+7")
rect(7,"terminal-tab",(402,633,88,37),radius=(4,10),border=False,sample=(446,638),padding="label left 17; underline y=666..669")
rect(7,"terminal-tab-underline",(406,666,80,3),border=False,sample=(450,667),padding="0")
for name,label,x,w in [("terminal","Terminal",419,59),("output","Output",515,53),("problems","Problems",594,70),("debug-console","Debug Console",728,106),("ports","Ports",865,40)]:
    text(7,"panel-tab-"+name,label,(x,639,w,23),"ui")
rect(7,"problem-count-pill",(668,637,21,22),radius=(7,12),border=False,sample=(670,648),padding="0")
text(7,"problem-count","0",(674,639,13,20),"small-ui")
rect(7,"terminal-run-button",(1004,633,88,30),radius=(4,10),sample=(1084,639),padding="play left 11; label x+33; chevron right 9")
icon(7,"terminal-run",(1015,639,17,20))
text(7,"terminal-run-label","Run",(1037,639,30,21),"ui")
icon(7,"terminal-run-chevron",(1068,642,16,13))
icon(7,"terminal-split",(1104,637,22,22))
icon(7,"terminal-trash",(1143,637,22,23))
icon(7,"terminal-more",(1181,641,22,14))
text(7,"terminal-prompt","forge-room-app git:(feature/room-collab) ✗ npm run dev",(443,685,460,23),"terminal-prompt",annotate=True)
icon(7,"terminal-prompt-dot",(418,688,14,14),color="green")
text(7,"terminal-package","> forge-room-app@0.1.0 dev",(420,716,252,23),"terminal")
text(7,"terminal-vite","> vite",(420,737,74,20),"terminal")
text(7,"terminal-ready","VITE v5.4.2   ready in 412 ms",(442,766,258,22),"terminal")
icon(7,"terminal-ready-check",(418,767,19,18),color="green")
icon(7,"terminal-local-arrow",(425,797,20,15),color="blue")
icon(7,"terminal-network-arrow",(425,818,20,15),color="blue")
text(7,"terminal-local-label","Local:",(453,793,62,23),"terminal")
text(7,"terminal-local-url","http://localhost:5173",(548,793,168,23),"terminal-link")
text(7,"terminal-network-label","Network:",(453,814,82,23),"terminal")
text(7,"terminal-network-url","http://192.168.1.24:5173",(548,814,195,23),"terminal")
rect(7,"current-branch-card",(977,681,230,155),radius=(5,12),sample=(990,688),padding="title left 12; internal dividers y=711 and y=752",note="Card is inside the terminal pane at right; left splitter at x=966.")
text(7,"branch-card-heading","Current Branch",(989,688,112,23),"ui")
icon(7,"branch-card-branch",(990,721,20,23))
text(7,"branch-card-name","feature/room-collab",(1017,720,145,24),"ui")
icon(7,"branch-card-check",(1173,721,21,19),color="green")
icon(7,"branch-ahead-behind",(992,760,23,21))
text(7,"branch-ahead","Ahead 2 ↓ Behind 0",(1023,760,148,23),"ui")
rect(7,"create-pr-button",(988,791,208,35),radius=(4,10),sample=(1190,799),padding="label centered left; external link right 9")
text(7,"create-pr-label","Create Pull Request...",(1013,800,148,23),"ui")
icon(7,"create-pr-external",(1171,799,21,21))

# 8. Dock.
rect(8,"dock-tabs",(1220,77,427,49),border=False,sample=(1580,103),padding="active tab x+4")
rect(8,"ai-tab",(1224,81,156,45),radius=(5,13),border=False,sample=(1300,86),padding="sparkle left 17; label x+47")
rect(8,"ai-tab-underline",(1227,122,149,3),border=False,sample=(1320,123),padding="0")
icon(8,"ai-tab-sparkle",(1238,90,23,23),color="red")
text(8,"ai-tab-label","AI Assistant",(1271,92,92,22),"tab")
icon(8,"room-tab-people",(1407,91,25,23))
text(8,"dock-room-label","Room",(1438,92,50,22),"tab")
icon(8,"dock-more",(1603,95,24,14))
icon(8,"assistant-brand-logo",(1243,147,40,40),color="multi",annotate=True)
text(8,"assistant-brand-title","Forge Room AI",(1298,146,122,24),"section-heading")
text(8,"assistant-brand-subtitle","Agentic coding in your workspace",(1298,170,240,23),"small-ui")
rect(8,"quick-actions-card",(1235,209,402,150),radius=(7,16),sample=(1590,222),padding="heading left 15, top 15; button grid left 13, right 13; gap 11 horizontally, 6 vertically")
text(8,"quick-actions-heading","How can I help you today?",(1249,223,226,26),"card-heading")
for id,label,b,tx,roi in [
    ("implement","Implement a feature",(1248,261,182,40),1288,(1261,271,23,22)),
    ("explain","Explain this code",(1441,261,183,40),1495,(1457,269,24,25)),
    ("tests","Write tests",(1248,307,182,39),1302,(1264,312,24,26)),
    ("fix","Find and fix issues",(1441,307,183,39),1493,(1457,312,25,26)),
]:
    rect(8,"quick-action-"+id,b,radius=(5,12),sample=(b[0]+b[2]-8,b[1]+7),padding="icon and label centered as a group",annotate=False)
    icon(8,"quick-action-icon-"+id,roi)
    text(8,"quick-action-label-"+id,label,(tx,b[1]+10,b[0]+b[2]-tx-5,23),"small-ui")
rect(8,"user-message-bubble",(1285,375,352,63),radius=(7,16),border=False,sample=(1600,410),padding="left 16; top 10; line advance 23",note="Small left pointer toward the avatar; no border visible.")
icon(8,"user-avatar",(1238,383,38,37),note="Circular dark avatar behind a head-and-shoulders outline.",annotate=True)
text(8,"user-message-line-1","Update the Room component to show",(1301,384,305,23),"chat-body")
text(8,"user-message-line-2","agent presence and a live status badge.",(1301,407,319,23),"chat-body")
icon(8,"assistant-message-logo",(1241,454,33,33),color="multi",annotate=True)
text(8,"assistant-message-line-1","I’ll update the Room component to include",(1285,454,338,23),"chat-body")
text(8,"assistant-message-line-2","agent presence with real-time status indicators.",(1285,477,350,23),"chat-body")
rect(8,"edited-summary",(1285,510,352,82),radius=(6,13),sample=(1624,519),padding="checks left 16; label left 43; filename left 98; counts right 14; divider y=550")
for name,file,plus,minus,cy,chip in [
    ("room","Room.tsx","+42","−6",532,(1383,517,81,27)),
    ("presence","AgentPresence.tsx","+98","−2",570,(1383,555,147,27)),
]:
    icon(8,"edited-check-"+name,(1298,cy-10,23,21),color="green")
    text(8,"edited-label-"+name,"Edited",(1328,cy-10,51,23),"ui")
    rect(8,"filename-chip-"+name,chip,radius=(3,9),sample=(chip[0]+chip[2]-7,chip[1]+5),padding="text left 11; top 5",annotate=False)
    text(8,"edited-file-"+name,file,(1394,cy-11,chip[2]-16,23),"ui")
    text(8,"edited-plus-"+name,plus,(1571,cy-10,31,23),"diff-green")
    text(8,"edited-minus-"+name,minus,(1604,cy-10,25,23),"diff-red")
rect(8,"composer",(1237,617,400,124),radius=(7,16),sample=(1244,679),padding="text left 16, top 14; line advance 23; bottom tools center near y=713; send right 9")
text(8,"composer-placeholder-1","Ask anything, or use @ to reference files,",(1253,629,329,24),"placeholder")
text(8,"composer-placeholder-2","agents, or Room context...",(1253,652,246,24),"placeholder")
icon(8,"composer-plus",(1256,702,22,22))
icon(8,"composer-at",(1296,700,25,25))
icon(8,"composer-cube",(1337,700,26,26))
text(8,"composer-model","Forge AI Pro",(1473,703,80,23),"ui")
icon(8,"composer-model-chevron",(1556,706,17,14))
rect(8,"send-button",(1587,695,42,36),radius=(5,12),sample=(1592,702),padding="paper plane centered",note="Red area is visibly nonuniform; sample is a composite color, source gradient unknown.")
icon(8,"send-plane",(1597,700,24,25))

# 9. Status bar. Keep all visible items, not just a generic status template.
rect(9,"status-band",(29,862,1618,41),border=False,sample=(1100,892),padding="left item x+23; common visual center y=882")
STATUS = [
    ("branch", "feature/room-collab", (80,872,133,23), (51,871,21,22)),
    ("sync", "2 ↑ 0 ↓", (292,872,48,23), (262,871,22,22)),
    ("pull-request", "PR #142", (406,872,57,23), (378,871,22,22)),
    ("room", "Room: Product Launch", (685,872,146,23), (654,869,23,25)),
    ("members", "4 members · 2 agents", (869,872,145,23), (849,876,14,14)),
    ("position", "Ln 14, Col 52", (1190,872,94,23), None),
    ("encoding", "UTF-8", (1316,872,50,23), None),
    ("line-ending", "CRLF", (1387,872,39,23), None),
    ("model", "Forge AI Pro", (1535,872,88,23), (1505,871,25,23)),
]
for id,label,b,ir in STATUS:
    text(9,"status-"+id,label,b,"status")
    if ir: icon(9,"status-icon-"+id,ir,color="green" if id=="members" else "red" if id=="room" else "neutral")
icon(9,"status-branch-chevron",(217,876,15,13))
icon(9,"status-home",(1453,870,25,24))
rect(9,"review-needed-pill",(470,870,108,24),radius=(7,14),border=False,sample=(573,881),padding="text left 10, top 4")
text(9,"review-needed-label","Review needed",(480,873,92,21),"small-ui")

SEPARATORS = [
    (2,"title-room-run-separator",(1322,37,1,24)),(2,"title-controls-separator",(1484,36,1,27)),
    (4,"explorer-actions-divider",(327,91,1,22)),(4,"explorer-header-divider",(106,124,281,2)),
    (5,"scm-header-divider",(106,573,281,2)),(5,"commit-segment-divider",(326,716,1,31)),
    (6,"tab-agent-divider",(749,91,1,22)),(6,"tab-schema-divider",(956,91,1,22)),
    (7,"terminal-card-splitter",(966,670,1,191)),(7,"branch-heading-divider",(979,711,226,1)),
    (7,"branch-status-divider",(979,751,226,1)),(8,"edited-row-divider",(1290,549,341,1)),
    (9,"status-separator-1",(245,872,1,21)),(9,"status-separator-2",(357,872,1,21)),
    (9,"status-separator-3",(635,872,1,21)),(9,"status-separator-4",(1490,872,1,21)),
]
for area,id,b in SEPARATORS:
    rect(area,id,b,border=False,padding="0",sample=(b[0],b[1]+b[3]//2),note="Measured visible separator band.",annotate=False)

# Font sizes here are reconstruction settings, separate from the exact ink masks.
STYLES = {
    "brand":("Segoe UI",19.25,600,"brand"),
    "panel-heading":("Segoe UI",16,600,"panel-heading"),
    "section-heading":("Segoe UI",16.5,600,"section-heading"),
    "tab":("Segoe UI",16,400,"tab"),
    "ui":("Segoe UI",14,400,"tab"),
    "tree":("Segoe UI",14.5,400,"tree"),
    "small-ui":("Segoe UI",14,400,"tree"),
    "small-helper":("Segoe UI",13,400,"small-helper"),
    "card-heading":("Segoe UI",18,600,"card-heading"),
    "chat-body":("Segoe UI",16,400,"chat-body"),
    "placeholder":("Segoe UI",16,400,"chat-body"),
    "breadcrumb":("Segoe UI",14,400,"breadcrumb"),
    "code":("Consolas",15.5,400,"code"),
    "line-number":("Consolas",14,400,"code"),
    "line-number-active":("Consolas",14,400,"code"),
    "terminal":("Consolas",15,400,"terminal"),
    "terminal-link":("Consolas",15,400,"terminal"),
    "terminal-prompt":("Consolas",15,600,"terminal"),
    "status":("Segoe UI",14,400,"status"),
    "badge-blue":("Segoe UI",14,400,"tree"),
    "badge-red":("Segoe UI",14,400,"tree"),
    "diff-green":("Segoe UI",14,400,"tree"),
    "diff-red":("Segoe UI",14,400,"tree"),
}
