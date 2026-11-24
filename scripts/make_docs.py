# Generates the project presentation (.pptx) and report (.docx) from the
# repository's real screenshots and measured results.
# Run: python scripts/make_docs.py
from pathlib import Path

from PIL import Image
from docx import Document
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.shared import Inches as DInches
from docx.shared import Pt as DPt
from docx.shared import RGBColor as DRGB
from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE
from pptx.enum.text import PP_ALIGN
from pptx.util import Inches, Pt

ROOT = Path(__file__).resolve().parent.parent
SHOTS = ROOT / "docs" / "screenshots"
OUT_PPT = ROOT / "docs" / "presentation" / "MiniOSKernel_Simulator.pptx"
OUT_DOCX = ROOT / "docs" / "report" / "MiniOSKernel_Simulator_Report.docx"
REPO = "https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator"

# Dark terminal palette
BG = RGBColor(0x0C, 0x0C, 0x0C)
PANEL = RGBColor(0x1C, 0x1C, 0x1C)
WHITE = RGBColor(0xF2, 0xF2, 0xF2)
TEXT = RGBColor(0xD8, 0xD8, 0xD8)
MUTED = RGBColor(0x9A, 0x9A, 0x9A)
CYAN = RGBColor(0x56, 0xB6, 0xC2)
GREEN = RGBColor(0x98, 0xC3, 0x79)
YELLOW = RGBColor(0xE5, 0xC0, 0x7B)
RED = RGBColor(0xE0, 0x6C, 0x75)
MAGENTA = RGBColor(0xC6, 0x78, 0xDD)
BLUE = RGBColor(0x61, 0xAF, 0xEF)

FONT = "Segoe UI"
MONO = "Consolas"

SLIDE_W = Inches(13.333)
SLIDE_H = Inches(7.5)


# --------------------------------------------------------------------------
# presentation helpers
# --------------------------------------------------------------------------

def new_slide(prs):
    slide = prs.slides.add_slide(prs.slide_layouts[6])
    fill = slide.background.fill
    fill.solid()
    fill.fore_color.rgb = BG
    return slide


def textbox(slide, left, top, width, height, text, size=18, color=TEXT, bold=False,
            font=FONT, align=PP_ALIGN.LEFT, anchor=None):
    box = slide.shapes.add_textbox(left, top, width, height)
    tf = box.text_frame
    tf.word_wrap = True
    if anchor is not None:
        tf.vertical_anchor = anchor
    p = tf.paragraphs[0]
    p.alignment = align
    run = p.add_run()
    run.text = text
    run.font.size = Pt(size)
    run.font.bold = bold
    run.font.color.rgb = color
    run.font.name = font
    return box


def bullets(slide, left, top, width, height, items, size=16, color=TEXT, gap=8):
    box = slide.shapes.add_textbox(left, top, width, height)
    tf = box.text_frame
    tf.word_wrap = True
    first = True
    for item in items:
        p = tf.paragraphs[0] if first else tf.add_paragraph()
        first = False
        p.space_after = Pt(gap)
        if isinstance(item, tuple):
            label, body = item
            r1 = p.add_run()
            r1.text = f"• {label}  "
            r1.font.size = Pt(size)
            r1.font.bold = True
            r1.font.color.rgb = CYAN
            r1.font.name = FONT
            r2 = p.add_run()
            r2.text = body
            r2.font.size = Pt(size)
            r2.font.color.rgb = color
            r2.font.name = FONT
        else:
            run = p.add_run()
            run.text = f"• {item}"
            run.font.size = Pt(size)
            run.font.color.rgb = color
            run.font.name = FONT
    return box


def slide_header(slide, kicker, title):
    bar = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.55), Inches(0.55), Inches(0.09), Inches(0.62))
    bar.fill.solid()
    bar.fill.fore_color.rgb = CYAN
    bar.line.fill.background()
    textbox(slide, Inches(0.8), Inches(0.45), Inches(9.5), Inches(0.35), kicker, 12, MUTED)
    textbox(slide, Inches(0.8), Inches(0.72), Inches(11.6), Inches(0.75), title, 30, WHITE, bold=True)


def footer(slide, number, dark=True):
    line = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.55), Inches(7.02), Inches(12.2), Inches(0.02))
    line.fill.solid()
    line.fill.fore_color.rgb = PANEL
    line.line.fill.background()
    textbox(slide, Inches(0.55), Inches(7.08), Inches(9.0), Inches(0.3),
            "Mini OS Kernel Simulator  ·  Rust  ·  " + REPO,
            9, MUTED)
    textbox(slide, Inches(12.2), Inches(7.08), Inches(0.6), Inches(0.3), str(number), 9, MUTED, align=PP_ALIGN.RIGHT)


def add_picture_fit(slide, name, right_margin=0.55, top=1.7, max_w=5.2, max_h=5.1):
    path = SHOTS / name
    with Image.open(path) as im:
        w, h = im.size
    aspect = w / h
    width = max_w
    height = width / aspect
    if height > max_h:
        height = max_h
        width = height * aspect
    left = SLIDE_W - Inches(right_margin) - Inches(width)
    slide.shapes.add_picture(str(path), left, Inches(top), width=Inches(width), height=Inches(height))


def panel(slide, left, top, width, height, fill=PANEL):
    box = slide.shapes.add_shape(MSO_SHAPE.ROUNDED_RECTANGLE, left, top, width, height)
    box.fill.solid()
    box.fill.fore_color.rgb = fill
    box.line.color.rgb = PANEL
    box.shadow.inherit = False
    return box


def code_box(slide, left, top, width, height, lines, size=12):
    box = slide.shapes.add_textbox(left, top, width, height)
    tf = box.text_frame
    tf.word_wrap = True
    first = True
    for line in lines:
        p = tf.paragraphs[0] if first else tf.add_paragraph()
        first = False
        run = p.add_run()
        run.text = line
        run.font.size = Pt(size)
        run.font.name = MONO
        run.font.color.rgb = GREEN if line.strip().startswith(("fn ", "pub")) else TEXT
    return box


def comparison_table(slide, left, top, width):
    rows = [
        ("ALGORITHM", "WAIT", "TURNAROUND", "RESPONSE"),
        ("FCFS", "8.75", "15.25", "8.75"),
        ("SJF", "7.75", "14.25", "7.75"),
        ("SRTF", "6.50", "13.00", "4.25"),
        ("ROUND ROBIN q=2", "12.75", "19.25", "2.00"),
        ("PRIORITY", "10.25", "16.75", "10.25"),
    ]
    table = slide.shapes.add_table(len(rows), 4, left, top, width, Inches(2.2)).table
    widths = [2.4, 0.9, 1.3, 1.0]
    for i, wd in enumerate(widths):
        table.columns[i].width = Inches(wd)
    for r, row in enumerate(rows):
        for c, value in enumerate(row):
            cell = table.cell(r, c)
            cell.fill.solid()
            cell.fill.fore_color.rgb = PANEL if r else RGBColor(0x24, 0x3A, 0x3D)
            p = cell.text_frame.paragraphs[0]
            run = p.add_run()
            run.text = value
            run.font.size = Pt(12)
            run.font.name = MONO
            run.font.color.rgb = GREEN if (r == 3 and c > 0) else (WHITE if r == 0 else TEXT)
            run.font.bold = r == 0 or r == 3
    note = slide.shapes.add_textbox(left, top + Inches(2.45), width, Inches(0.3))
    para = note.text_frame.paragraphs[0]
    run = para.add_run()
    run.text = "Green row: SRTF produced the best wait and turnaround in this run."
    run.font.size = Pt(11)
    run.font.color.rgb = MUTED


# --------------------------------------------------------------------------
# build presentation
# --------------------------------------------------------------------------

def build_ppt():
    prs = Presentation()
    prs.slide_width = SLIDE_W
    prs.slide_height = SLIDE_H

    # 1 title
    slide = new_slide(prs)
    accent = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(0.55), Inches(2.25), Inches(0.12), Inches(2.05))
    accent.fill.solid()
    accent.fill.fore_color.rgb = CYAN
    accent.line.fill.background()
    textbox(slide, Inches(0.95), Inches(2.15), Inches(11.0), Inches(0.5), "OPERATING SYSTEMS PROJECT", 14, CYAN, bold=True)
    textbox(slide, Inches(0.95), Inches(2.6), Inches(11.5), Inches(0.9), "Mini OS Kernel Simulator", 44, WHITE, bold=True)
    textbox(slide, Inches(0.95), Inches(3.45), Inches(10.5), Inches(0.5),
            "A discrete event kernel simulator written in Rust", 20, MUTED)
    textbox(slide, Inches(0.95), Inches(4.35), Inches(8.0), Inches(0.4), "Gunjan   ·   November 2026", 14, TEXT)
    textbox(slide, Inches(0.95), Inches(5.0), Inches(11.5), Inches(0.4), REPO, 12, GREEN, font=MONO)
    add_picture_fit(slide, "01-main-menu.png", right_margin=0.55, top=4.35, max_w=4.4, max_h=2.2)
    textbox(slide, Inches(0.95), Inches(6.75), Inches(9.0), Inches(0.3),
            "68 automated tests  ·  CI on Linux, Windows and macOS  ·  zero external dependencies", 11, MUTED)

    # 2 what it is
    slide = new_slide(prs)
    slide_header(slide, "OVERVIEW", "What the simulator does")
    bullets(slide, Inches(0.8), Inches(1.8), Inches(6.6), Inches(4.9), [
        ("Process management", "process table, validated states, event log"),
        ("CPU scheduling", "five algorithms, Gantt charts, full metrics"),
        ("Memory", "First Fit, Best Fit, Worst Fit, paging, replacement"),
        ("Synchronization", "race demos, mutex, producer consumer"),
        ("IPC", "bounded message queues and shared memory"),
        ("Deadlock", "Banker's Algorithm and deadlock detection"),
        ("System calls", "validated kernel entry points"),
    ], size=16)
    add_picture_fit(slide, "02-process-management.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 2)

    # 3 goals
    slide = new_slide(prs)
    slide_header(slide, "MOTIVATION", "Why build a kernel simulator")
    cards = [
        ("Show, do not tell", "Every OS concept is a working implementation you can drive from a menu.", CYAN),
        ("Deterministic demos", "Ticks, not sleeps. The same workload always produces the same numbers.", GREEN),
        ("Verifiable results", "Hand computed examples are locked in as automated tests.", YELLOW),
        ("Clear boundaries", "One kernel owner, typed errors, UI separated from the engine.", MAGENTA),
    ]
    x = Inches(0.8)
    for title, body, color in cards:
        panel(slide, x, Inches(2.2), Inches(2.9), Inches(3.4))
        textbox(slide, x + Inches(0.2), Inches(2.45), Inches(2.5), Inches(0.6), title, 16, color, bold=True)
        textbox(slide, x + Inches(0.2), Inches(3.15), Inches(2.5), Inches(2.2), body, 13, TEXT)
        x += Inches(3.05)
    textbox(slide, Inches(0.8), Inches(6.0), Inches(11.8), Inches(0.6),
            "The simulator is not a bootable kernel. It models kernel behavior in a controlled environment.", 13, MUTED)
    footer(slide, 3)

    # 4 architecture
    slide = new_slide(prs)
    slide_header(slide, "ARCHITECTURE", "Layered design with a single kernel owner")
    boxes = [
        ("USER INTERFACE", "menus, tables, dashboard", CYAN),
        ("KERNEL", "owns process table, memory, paging, IPC, lock state, events", WHITE),
        ("SYSTEM CALL API", "fork, exit, wait, allocate, free, send, receive", YELLOW),
        ("EVENT LOG", "every action advances the simulation tick", GREEN),
    ]
    y = Inches(1.75)
    for label, sub, color in boxes:
        shp = slide.shapes.add_shape(MSO_SHAPE.ROUNDED_RECTANGLE, Inches(1.2), y, Inches(5.3), Inches(1.0))
        shp.fill.solid()
        shp.fill.fore_color.rgb = PANEL
        shp.line.color.rgb = color
        tf = shp.text_frame
        tf.word_wrap = True
        p = tf.paragraphs[0]
        r = p.add_run()
        r.text = label
        r.font.size = Pt(15)
        r.font.bold = True
        r.font.color.rgb = color
        p2 = tf.add_paragraph()
        r2 = p2.add_run()
        r2.text = sub
        r2.font.size = Pt(11)
        r2.font.color.rgb = TEXT
        y += Inches(1.22)
    textbox(slide, Inches(7.0), Inches(1.8), Inches(5.6), Inches(4.6), "", 12, TEXT)
    bullets(slide, Inches(7.0), Inches(1.8), Inches(5.5), Inches(4.9), [
        ("Ownership", "one Kernel struct, no global mutable state"),
        ("Errors", "typed Result values from every subsystem"),
        ("Testability", "schedulers are pure functions over snapshots"),
        ("Model", "discrete event simulation with ticks"),
    ], size=14)
    footer(slide, 4)

    # 5 processes
    slide = new_slide(prs)
    slide_header(slide, "MODULE 1", "Process management")
    bullets(slide, Inches(0.8), Inches(1.75), Inches(6.3), Inches(4.9), [
        ("PCB fields", "pid, name, state, arrival, burst, remaining, priority, memory, stats"),
        ("States", "NEW, READY, RUNNING, WAITING, TERMINATED"),
        ("Validation", "illegal transitions are rejected with a clear error"),
        ("Operations", "fork, exit, wait, wake, dispatch, inspect"),
        ("Event log", "every transition is recorded with a tick"),
    ], size=15)
    code_box(slide, Inches(0.9), Inches(4.55), Inches(6.2), Inches(1.6), [
        "NEW → READY → RUNNING → TERMINATED",
        "RUNNING → WAITING → READY resumes",
        "WAITING → RUNNING is rejected by the kernel",
    ], size=12)
    add_picture_fit(slide, "02-process-management.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 5)

    # 6 scheduling
    slide = new_slide(prs)
    slide_header(slide, "MODULE 2", "CPU scheduling, five algorithms")
    bullets(slide, Inches(0.8), Inches(1.75), Inches(6.4), Inches(4.6), [
        ("FCFS", "arrival order"),
        ("SJF", "shortest job first, non preemptive"),
        ("SRTF", "shortest remaining time, preemptive"),
        ("Round Robin", "fixed time quantum"),
        ("Priority", "lower value runs first"),
        ("Metrics", "wait, turnaround, response, CPU utilization, throughput"),
    ], size=15)
    textbox(slide, Inches(0.85), Inches(5.55), Inches(6.3), Inches(1.0),
            "Each run renders a Gantt chart with one color per process, plus a per process statistics table.",
            12, MUTED)
    add_picture_fit(slide, "03-srtf-scheduling.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 6)

    # 7 comparison
    slide = new_slide(prs)
    slide_header(slide, "MODULE 2", "Same workload, five policies side by side")
    comparison_table(slide, Inches(0.8), Inches(1.75), Inches(5.7))
    textbox(slide, Inches(0.8), Inches(4.6), Inches(5.9), Inches(1.9),
            "Measured on four processes with arrivals 0, 1, 2, 3 and bursts 8, 4, 9, 5. "
            "SRTF minimizes average wait and turnaround. Round Robin trades wait time for "
            "the lowest response time and fairness. The comparison is computed on process "
            "snapshots, so the table stays valid while switching algorithms.", 12, TEXT)
    add_picture_fit(slide, "04-algorithm-comparison.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 7)

    # 8 memory
    slide = new_slide(prs)
    slide_header(slide, "MODULE 3", "Contiguous memory management")
    bullets(slide, Inches(0.8), Inches(1.75), Inches(6.4), Inches(4.9), [
        ("Strategies", "First Fit, Best Fit, Worst Fit, switchable at runtime"),
        ("Block list", "start sorted, split on allocation"),
        ("Coalescing", "adjacent free blocks merge on release"),
        ("Reporting", "used, free, largest hole, external fragmentation"),
        ("Map", "colored block map with owner names"),
    ], size=15)
    add_picture_fit(slide, "05-memory-map.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 8)

    # 9 paging
    slide = new_slide(prs)
    slide_header(slide, "MODULE 3", "Paging and page replacement")
    bullets(slide, Inches(0.8), Inches(1.75), Inches(6.4), Inches(3.6), [
        ("Translation", "physical = frame × page size + offset"),
        ("Page faults", "validated, counted, displayed per process"),
        ("Policies", "FIFO, LRU, Optimal"),
        ("Optimal", "computed offline over a known reference string"),
    ], size=15)
    textbox(slide, Inches(0.85), Inches(4.75), Inches(6.2), Inches(1.9),
            "Belady reference string: 1 2 3 4 1 2 5 1 2 3 4 5 with 3 frames. "
            "FIFO gives 9 faults, LRU gives 10, Optimal gives 7. The same string with 4 "
            "frames gives 10 FIFO faults, a live demonstration of Belady's anomaly.", 12, TEXT)
    add_picture_fit(slide, "07-page-replacement.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 9)

    # 10 sync
    slide = new_slide(prs)
    slide_header(slide, "MODULE 4", "Synchronization")
    bullets(slide, Inches(0.8), Inches(1.75), Inches(6.4), Inches(4.9), [
        ("Deterministic race", "two simulated loads, one lost update"),
        ("Real thread race", "four threads increment a raw counter"),
        ("Mutex fix", "Arc and Mutex produce the exact count"),
        ("Producer consumer", "bounded buffer with Condvar wakes"),
        ("Semaphore", "P and V walkthrough for logical processes"),
    ], size=15)
    add_picture_fit(slide, "08-synchronization.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 10)

    # 11 ipc
    slide = new_slide(prs)
    slide_header(slide, "MODULE 5", "Inter process communication")
    bullets(slide, Inches(0.8), Inches(1.75), Inches(6.4), Inches(4.9), [
        ("Queues", "bounded capacity, FIFO delivery"),
        ("Blocked senders", "full queue records the sender, receive wakes it"),
        ("Shared memory", "byte regions with bounds checking"),
        ("Events", "sends, receives and wakes appear in the log"),
    ], size=15)
    add_picture_fit(slide, "09-ipc.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 11)

    # 12 deadlock
    slide = new_slide(prs)
    slide_header(slide, "MODULE 6", "Deadlock avoidance and detection")
    bullets(slide, Inches(0.8), Inches(1.75), Inches(6.4), Inches(4.9), [
        ("Banker's Algorithm", "safety check with safe sequence"),
        ("Requests", "granted only when the state stays safe"),
        ("Rollback", "unsafe grants are reverted and reported"),
        ("Detection", "work and finish sweep over the request matrix"),
        ("Examples", "Silberschatz safe and deadlocked scenarios built in"),
    ], size=15)
    add_picture_fit(slide, "10-deadlock-banker.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 12)

    # 13 system calls + events
    slide = new_slide(prs)
    slide_header(slide, "MODULE 7", "System calls and the event log")
    code_box(slide, Inches(0.9), Inches(1.8), Inches(6.4), Inches(3.3), [
        "fork_process(name, arrival, burst, priority, memory)",
        "exit_process(pid)",
        "wait_process(pid)",
        "allocate_memory(pid, size)",
        "free_memory(pid)",
        "send_message(sender, queue, receiver, body)",
        "receive_message(queue, receiver)",
    ], size=13)
    bullets(slide, Inches(0.85), Inches(5.15), Inches(6.4), Inches(1.6), [
        ("Validation", "unknown pid, bad state, low memory, full queue"),
        ("Logging", "[12] CPU assigned to P2, ticked and centralized"),
    ], size=13)
    add_picture_fit(slide, "12-kernel-dashboard.png", right_margin=0.5, top=1.75, max_w=5.2, max_h=4.9)
    footer(slide, 13)

    # 14 quality
    slide = new_slide(prs)
    slide_header(slide, "QUALITY", "Tests and engineering discipline")
    stats = [
        ("68", "automated tests", GREEN),
        ("3", "CI operating systems", CYAN),
        ("0", "external dependencies", YELLOW),
        ("100%", "Rust standard library", MAGENTA),
    ]
    x = Inches(0.8)
    for value, label, color in stats:
        panel(slide, x, Inches(1.9), Inches(2.9), Inches(1.9))
        textbox(slide, x + Inches(0.2), Inches(2.05), Inches(2.5), Inches(0.8), value, 36, color, bold=True)
        textbox(slide, x + Inches(0.2), Inches(2.9), Inches(2.6), Inches(0.7), label, 13, TEXT)
        x += Inches(3.05)
    bullets(slide, Inches(0.85), Inches(4.2), Inches(11.5), Inches(2.3), [
        ("Scheduler tests", "hand computed timelines and metrics for all five algorithms"),
        ("Memory tests", "allocation, coalescing, Belady counts, anomaly check"),
        ("Deadlock tests", "safe and unsafe states, sequence validation, rollback"),
        ("IPC and sync tests", "FIFO delivery, blocked senders, no lost messages"),
    ], size=14)
    footer(slide, 14)

    # 15 run
    slide = new_slide(prs)
    slide_header(slide, "GETTING STARTED", "How to run the simulator")
    panel(slide, Inches(0.8), Inches(1.75), Inches(5.8), Inches(2.2))
    textbox(slide, Inches(1.0), Inches(1.9), Inches(5.4), Inches(0.4), "Installed on this machine (npm)", 12, MUTED)
    code_box(slide, Inches(1.0), Inches(2.3), Inches(5.4), Inches(1.4), [
        "mini-os-kernel",
    ], size=16)
    panel(slide, Inches(6.9), Inches(1.75), Inches(5.7), Inches(2.2))
    textbox(slide, Inches(7.1), Inches(1.9), Inches(5.3), Inches(0.4), "From a clone", 12, MUTED)
    code_box(slide, Inches(7.1), Inches(2.3), Inches(5.3), Inches(1.4), [
        "git clone " + REPO,
        "cd Mini-OS-Kernel-Simulator",
        "cargo run --release",
    ], size=11)
    bullets(slide, Inches(0.85), Inches(4.35), Inches(11.6), Inches(2.3), [
        ("npm", "npm install -g mini-os-kernel-simulator"),
        ("cargo", "cargo install --git " + REPO + " mini-os-kernel"),
        ("Docker", "docker build . then docker run it mini-os-kernel"),
        ("Windows and Linux scripts", "install.ps1 and install.sh in the repository"),
    ], size=14)
    footer(slide, 15)

    # 16 demo script
    slide = new_slide(prs)
    slide_header(slide, "LIVE DEMO", "Five minutes with the professor")
    steps = [
        ("1", "Create four processes", "arrivals 0, 1, 2, 3 with bursts 8, 4, 9, 5 and memory needs"),
        ("2", "Run SRTF and Round Robin", "point at the colored Gantt chart, then Compare all"),
        ("3", "Memory and paging", "allocate blocks, free one, show the map, run the Belady demo"),
        ("4", "Synchronization", "run the real thread race, then the mutex result"),
        ("5", "Deadlock", "load the safe scenario, run Banker's, then the detection scenario"),
        ("6", "Finish on the dashboard", "tick count equals the number of logged kernel events"),
    ]
    y = Inches(1.8)
    for num, title, body in steps:
        shp = slide.shapes.add_shape(MSO_SHAPE.OVAL, Inches(0.9), y, Inches(0.5), Inches(0.5))
        shp.fill.solid()
        shp.fill.fore_color.rgb = CYAN
        shp.line.fill.background()
        tf = shp.text_frame
        p = tf.paragraphs[0]
        p.alignment = PP_ALIGN.CENTER
        r = p.add_run()
        r.text = num
        r.font.bold = True
        r.font.size = Pt(16)
        r.font.color.rgb = BG
        textbox(slide, Inches(1.6), y - Inches(0.02), Inches(2.9), Inches(0.4), title, 15, WHITE, bold=True)
        textbox(slide, Inches(4.6), y - Inches(0.02), Inches(7.9), Inches(0.65), body, 13, MUTED)
        y += Inches(0.82)
    footer(slide, 16)

    # 17 thank you
    slide = new_slide(prs)
    textbox(slide, Inches(1.0), Inches(2.2), Inches(11.2), Inches(1.0), "Thank You", 48, WHITE, bold=True)
    textbox(slide, Inches(1.0), Inches(3.15), Inches(11.2), Inches(0.5),
            "Questions and live demonstrations are welcome.", 18, MUTED)
    textbox(slide, Inches(1.0), Inches(4.2), Inches(11.2), Inches(0.4), "Repository:", 13, MUTED)
    textbox(slide, Inches(1.0), Inches(4.55), Inches(11.2), Inches(0.5), REPO, 18, GREEN, font=MONO)
    textbox(slide, Inches(1.0), Inches(5.6), Inches(11.2), Inches(0.4),
            "Mini OS Kernel Simulator  ·  Rust  ·  " + "68 tests, all green", 12, MUTED)

    OUT_PPT.parent.mkdir(parents=True, exist_ok=True)
    prs.save(str(OUT_PPT))
    print(f"wrote {OUT_PPT} ({len(prs.slides)} slides)")


# --------------------------------------------------------------------------
# report (docx)
# --------------------------------------------------------------------------

def add_heading(doc, text, level=1):
    p = doc.add_heading(text, level=level)
    for run in p.runs:
        run.font.name = "Segoe UI Semibold" if level == 1 else "Segoe UI"
        run.font.color.rgb = DRGB(0x1F, 0x6F, 0x78) if level == 1 else DRGB(0x2E, 0x7D, 0x6B)
    return p


def para(doc, text, size=11, color=None, bold=False, italic=False, align=None, space_after=6):
    p = doc.add_paragraph()
    run = p.add_run(text)
    run.font.size = DPt(size)
    run.font.name = "Segoe UI"
    run.font.bold = bold
    run.font.italic = italic
    if color:
        run.font.color.rgb = color
    if align is not None:
        p.alignment = align
    p.paragraph_format.space_after = DPt(space_after)
    p.paragraph_format.line_spacing = 1.15
    return p


def bullet(doc, label, body, size=10.5):
    p = doc.add_paragraph(style="List Bullet")
    r1 = p.add_run(label + ": ")
    r1.font.bold = True
    r1.font.size = DPt(size)
    r1.font.color.rgb = DRGB(0x1F, 0x6F, 0x78)
    r2 = p.add_run(body)
    r2.font.size = DPt(size)
    r2.font.name = "Segoe UI"
    p.paragraph_format.space_after = DPt(3)


def code_paragraph(doc, text):
    p = doc.add_paragraph()
    run = p.add_run(text)
    run.font.name = "Consolas"
    run.font.size = DPt(9)
    p.paragraph_format.space_after = DPt(2)
    p.paragraph_format.line_spacing = 1.0


def add_figure(doc, name, caption, width=6.2):
    path = SHOTS / name
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    p.add_run().add_picture(str(path), width=DInches(width))
    cap = doc.add_paragraph()
    cap.alignment = WD_ALIGN_PARAGRAPH.CENTER
    run = cap.add_run(caption)
    run.font.size = DPt(9)
    run.font.italic = True
    run.font.color.rgb = DRGB(0x66, 0x66, 0x66)


def build_report():
    doc = Document()
    style = doc.styles["Normal"]
    style.font.name = "Segoe UI"
    style.font.size = DPt(10.5)

    # cover
    for _ in range(4):
        doc.add_paragraph()
    para(doc, "MINI OS KERNEL SIMULATOR", size=26, bold=True, align=WD_ALIGN_PARAGRAPH.CENTER, color=DRGB(0x0C, 0x4F, 0x5C))
    para(doc, "A discrete event kernel simulator written in Rust", size=13, align=WD_ALIGN_PARAGRAPH.CENTER,
         color=DRGB(0x55, 0x55, 0x55))
    doc.add_paragraph()
    para(doc, "Project report", size=12, align=WD_ALIGN_PARAGRAPH.CENTER)
    para(doc, "Gunjan", size=12, bold=True, align=WD_ALIGN_PARAGRAPH.CENTER)
    para(doc, "November 2026", size=11, align=WD_ALIGN_PARAGRAPH.CENTER, color=DRGB(0x66, 0x66, 0x66))
    doc.add_paragraph()
    para(doc, "Repository: " + REPO, size=10, align=WD_ALIGN_PARAGRAPH.CENTER, color=DRGB(0x2E, 0x7D, 0x6B))
    doc.add_page_break()

    add_heading(doc, "Abstract", 1)
    para(doc, "This project implements a miniature operating system kernel simulator in Rust. "
              "It models the core responsibilities of a kernel: process management, CPU scheduling, "
              "memory management, paging, synchronization, inter process communication, deadlock "
              "avoidance and detection, and a simulated system call interface. The simulator is a "
              "discrete event system. Simulated processes advance by deterministic ticks, so every "
              "menu run is reproducible. The interface is a colorized terminal with menus, tables, "
              "charts and a dashboard. Correctness is guarded by 68 automated tests that encode hand "
              "computed schedules, known page replacement counts, and textbook deadlock examples.")
    para(doc, "The implementation uses only the Rust standard library and has zero external "
              "dependencies. Continuous integration runs the full test suite on Linux, Windows and "
              "macOS. The project is packaged for npm, cargo, Docker and one line installer scripts.")

    add_heading(doc, "1. Introduction", 1)
    para(doc, "Operating systems courses describe algorithms, but students rarely see a scheduler, "
              "an allocator and a deadlock detector cooperate inside one program. This project builds "
              "that environment. It is deliberately not a bootable kernel. Instead, it simulates the "
              "kernel's decisions and resource accounting so that every concept can be inspected, "
              "measured and demonstrated within seconds.")
    para(doc, "The goals were: show processes and their state machine, implement five CPU scheduling "
              "algorithms with metrics, implement contiguous allocation and paging with replacement, "
              "demonstrate synchronization problems and fixes, provide message passing and shared "
              "memory, implement Banker's Algorithm and deadlock detection, and route all operations "
              "through a validated system call layer with a central event log.")

    add_heading(doc, "2. Architecture", 1)
    para(doc, "One Kernel structure owns every subsystem: the process table, memory manager, paged "
              "memory, IPC state, deadlock scenario and event log. There is no global mutable state, "
              "and the user interface is separate from the simulation engine. Each subsystem returns "
              "typed errors that the kernel converts into one error enum. Every public kernel "
              "operation validates its arguments and logs an event, and every logged event advances "
              "the simulation tick.")
    para(doc, "Schedulers are pure functions over a snapshot of processes. This design has two "
              "benefits. First, the process table is never mutated by what if scheduling runs, so the "
              "same workload can be executed under all five algorithms and compared fairly. Second, "
              "each algorithm is directly testable without a terminal or kernel state.")
    para(doc, "The source layout follows these boundaries: process, scheduler, memory, paging, "
              "synchronization, IPC, deadlock, kernel and UI modules. Tests are separate integration "
              "suites per subsystem.")

    add_heading(doc, "3. Subsystems", 1)

    add_heading(doc, "3.1 Process management", 2)
    para(doc, "A process control block stores the pid, name, state, arrival time, burst time, "
              "remaining time, priority, memory requirement and accumulated statistics. States are "
              "NEW, READY, RUNNING, WAITING and TERMINATED. The legal transition graph is enforced by "
              "the kernel. NEW moves to READY, READY to RUNNING, RUNNING to READY, WAITING or "
              "TERMINATED, and WAITING returns to READY. Illegal requests are rejected with a "
              "descriptive error instead of being coerced. Every transition is logged with a tick.")

    add_heading(doc, "3.2 CPU scheduling", 2)
    para(doc, "Five algorithms are implemented: First Come First Serve, Shortest Job First, Shortest "
              "Remaining Time First, Round Robin with a configurable quantum, and non preemptive "
              "Priority scheduling where a lower value means higher priority. Each algorithm produces "
              "a timeline that is merged into Gantt intervals and colored per process.")
    para(doc, "Metrics are computed from the timeline: completion is the end of the last interval, "
              "turnaround is completion minus arrival, wait is turnaround minus burst, response is "
              "first start minus arrival, CPU utilization is busy time divided by makespan, and "
              "throughput is completed processes divided by makespan. The comparison view highlights "
              "the best value for every metric.")
    para(doc, "Measured example for processes with arrivals 0, 1, 2, 3 and bursts 8, 4, 9, 5: "
              "average wait is 8.75 for FCFS, 7.75 for SJF, 6.50 for SRTF, 12.75 for Round Robin "
              "with quantum 2, and 10.25 for Priority. SRTF wins on wait and turnaround; Round Robin "
              "wins on response time.")
    add_figure(doc, "03-srtf-scheduling.png", "Figure 1: SRTF run with colored Gantt chart, metrics and per process statistics.")

    add_heading(doc, "3.3 Contiguous memory management", 2)
    para(doc, "The allocator keeps a block list sorted by start address. First Fit, Best Fit and "
              "Worst Fit select a hole according to the active strategy. Allocation splits a hole and "
              "records the owner. Freeing a block marks it free and coalesces adjacent holes. The "
              "kernel reports used memory, free memory, the largest free hole and external "
              "fragmentation, and renders a colored map of physical memory.")
    add_figure(doc, "05-memory-map.png", "Figure 2: Best Fit allocation after freeing P2: the freed block and the tail hole remain visible.", width=5.6)

    add_heading(doc, "3.4 Paging and replacement", 2)
    para(doc, "Each process can be given a page table. Translation maps a logical address to a frame "
              "and an offset and validates the address against the process's page count. Page faults "
              "and hits are counted. FIFO, LRU and Optimal replacement are available. Optimal "
              "replacement requires future knowledge, so the true Optimal policy is computed offline "
              "over a known reference string, while online translation with the Optimal policy falls "
              "back to LRU; this limitation is documented in the code and the documentation.")
    para(doc, "The classic Belady reference string 1 2 3 4 1 2 5 1 2 3 4 5 with three frames "
              "produces 9 FIFO faults, 10 LRU faults and 7 Optimal faults. With four frames the same "
              "string produces 10 FIFO faults, which demonstrates Belady's anomaly. These exact "
              "numbers are asserted in the test suite.")
    add_figure(doc, "07-page-replacement.png", "Figure 3: Frame table and the FIFO, LRU and Optimal comparison for the Belady string.", width=5.0)

    add_heading(doc, "3.5 Synchronization", 2)
    para(doc, "Three demonstrations are provided. The first shows a deterministic interleaving where "
              "two loads read the same value and one store is lost. The second uses real OS threads "
              "that increment a shared counter through a raw pointer with no synchronization, then "
              "repeats the same workload with Arc and Mutex. The third is a bounded producer consumer "
              "pipeline built with a Mutex, a Condvar and a shared counter of pending sentinels; the "
              "test asserts that no message is lost or duplicated. A simulated semaphore reports "
              "would block instead of stalling real threads.")

    add_heading(doc, "3.6 Inter process communication", 2)
    para(doc, "Message queues have a bounded capacity. Sending to a full queue is refused and the "
              "sender is recorded as blocked; a successful receive frees a slot and wakes the longest "
              "blocked sender, and the wake is logged. Messages are delivered in FIFO order. Shared "
              "memory regions are byte vectors with bounds checked reads and writes. The menu shows "
              "queue capacity bars and message history.")

    add_heading(doc, "3.7 Deadlock management", 2)
    para(doc, "Banker's Algorithm checks whether the current state is safe and returns a safe "
              "sequence. A resource request is granted only if the resulting state remains safe; "
              "otherwise the change is rolled back and the request is denied. Deadlock detection uses "
              "the same work and finish sweep but examines outstanding requests instead of remaining "
              "claims, and reports the deadlocked processes. Both textbook scenarios are built into "
              "the menu.")
    add_figure(doc, "10-deadlock-banker.png", "Figure 4: Banker's Algorithm with a safe sequence and a granted resource request.", width=4.6)

    add_heading(doc, "3.8 System calls and event logging", 2)
    para(doc, "Seven calls form the kernel interface: fork process, exit process, wait process, "
              "allocate memory, free memory, send message and receive message. Every call validates "
              "the process id, the process state and the resource arguments before delegating to the "
              "responsible subsystem. All actions are written to a single event log with ticks, and "
              "the dashboard summarizes CPU state, queues, memory, paging and the current simulation "
              "time.")

    add_heading(doc, "4. Testing and correctness", 1)
    para(doc, "The project contains 68 integration tests across five suites plus the library build. "
              "Highlights:")
    bullet(doc, "Process tests", "creation, termination, legal and illegal transitions, failed fork behavior, page table validation.")
    bullet(doc, "Scheduler tests", "hand computed timelines and metrics for every algorithm, quantum clamping, zero burst and empty workloads.")
    bullet(doc, "Memory tests", "allocation success and failure, three fit strategies, coalescing, translation, FIFO and LRU and Optimal counts, Belady anomaly.")
    bullet(doc, "Deadlock tests", "safe and unsafe states, sequence validity, detection results, rollback on unsafe grant, inconsistent input handling.")
    bullet(doc, "IPC and sync tests", "FIFO delivery, blocked sender wake up, bounds checking, real thread mutex count, no lost producer consumer messages.")
    para(doc, "A continuous integration workflow builds and tests the project on Linux, Windows and "
              "macOS for every push. The repository history contains 60 small commits so the "
              "development process itself is inspectable.")

    add_heading(doc, "5. Installation and usage", 1)
    para(doc, "The simulator is a terminal application. Any of the following paths work:")
    code_paragraph(doc, "npm install -g mini-os-kernel-simulator     then run: mini-os-kernel")
    code_paragraph(doc, "npx mini-os-kernel-simulator")
    code_paragraph(doc, "cargo install --git " + REPO + " mini-os-kernel")
    code_paragraph(doc, "docker build .    then: docker run it mini-os-kernel")
    code_paragraph(doc, "git clone " + REPO + "    then: cargo run release")
    para(doc, "On this machine the npm package is already installed globally, so the command "
              "mini-os-kernel starts the simulator immediately. The main menu offers process "
              "management, scheduling, memory, synchronization, IPC, deadlock management, system "
              "calls and the dashboard. Input uses numbers and short text answers.")
    para(doc, "Colors are enabled automatically in a terminal and disabled when output is piped or "
              "when NO_COLOR is set, so scripts and logs remain plain text.")

    add_heading(doc, "6. Demonstration guide", 1)
    para(doc, "A five minute demonstration for a professor:")
    bullet(doc, "Preparation", "open Windows Terminal, run mini-os-kernel, resize so the Gantt charts and tables fit comfortably.")
    bullet(doc, "Processes", "create four processes with arrivals 0, 1, 2, 3, bursts 8, 4, 9, 5, priorities 1, 2, 1, 1 and memory needs 100, 80, 120, 60.")
    bullet(doc, "Scheduling", "run SRTF, point at the colored Gantt chart and the wait table, then run Compare all and explain why SRTF wins on wait while Round Robin wins on response.")
    bullet(doc, "Memory", "allocate blocks in the memory menu, free P2 and show the map and fragmentation, then attach a page table to P1, translate addresses and run the Belady replacement demo.")
    bullet(doc, "Synchronization", "run the real thread race twice, show that the racy result can be short, then show the mutex result always equals the expected count.")
    bullet(doc, "IPC", "create a queue of capacity two, send three messages, show the blocked sender, receive one and show the wake event.")
    bullet(doc, "Deadlock", "load the safe scenario and run Banker's Algorithm, request resources, then load the deadlock scenario and run detection.")
    bullet(doc, "Finish", "open the dashboard, show that system time equals the number of logged events, and mention the 68 tests and the CI matrix.")

    add_heading(doc, "7. Limitations and future work", 1)
    para(doc, "Simulated processes are data structures, not isolated user programs. Scheduling runs "
              "are what if snapshots and do not mutate process states. Memory is measured in abstract "
              "units. Online Optimal page replacement falls back to LRU because the future is unknown. "
              "The interface is text based.")
    para(doc, "Natural next steps are persistent scenario files, exporting schedule reports to CSV, a "
              "full screen terminal dashboard, an interactive resource allocation graph, and optional "
              "real thread backed simulated processes with a scheduler thread.")

    add_heading(doc, "References", 1)
    for ref in [
        "Silberschatz, Galvin and Gagne. Operating System Concepts, chapters on scheduling, memory, deadlock and synchronization.",
        "The Rust Programming Language, ownership and concurrency chapters.",
        "Belady, Nelson and Shedler. An anomaly in the incidence of page faults. 1969.",
        "Project repository, source code, tests, screenshots and development log: " + REPO,
    ]:
        p = doc.add_paragraph(ref, style="List Number")
        p.paragraph_format.space_after = DPt(2)

    add_heading(doc, "Appendix A: Menu map", 2)
    rows = [
        ("1", "Process Management", "fork, exit, wait, wake, tables, event log"),
        ("2", "CPU Scheduling", "five algorithms, Gantt, compare"),
        ("3", "Memory Management", "fit strategies, paging, replacement"),
        ("4", "Synchronization", "race demos, producer consumer, semaphore"),
        ("5", "IPC", "queues, shared memory"),
        ("6", "Deadlock", "Banker's, detection, requests"),
        ("7", "System Calls", "kernel entry points"),
        ("8", "Dashboard", "system overview"),
        ("9", "Exit", ""),
    ]
    table = doc.add_table(rows=len(rows) + 1, cols=3)
    table.style = "Light Grid Accent 1"
    hdr = table.rows[0].cells
    for i, title in enumerate(["Option", "Menu", "Contents"]):
        hdr[i].text = title
    for r, (opt, menu, contents) in enumerate(rows, start=1):
        cells = table.rows[r].cells
        cells[0].text = opt
        cells[1].text = menu
        cells[2].text = contents
    for row in table.rows:
        for cell in row.cells:
            for p in cell.paragraphs:
                for run in p.runs:
                    run.font.size = DPt(9)
                    run.font.name = "Segoe UI"

    OUT_DOCX.parent.mkdir(parents=True, exist_ok=True)
    doc.save(str(OUT_DOCX))
    print(f"wrote {OUT_DOCX}")


if __name__ == "__main__":
    build_ppt()
    build_report()
