"""OCR every second of the flight recording; write raw per-frame text to JSONL."""
import json, subprocess, sys, time
import numpy as np, cv2
from rapidocr_onnxruntime import RapidOCR

FF = r"C:\Users\BarronVonVuudmiester\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-9.0.2-full_build\bin\ffmpeg.exe"
VIDEO = r"C:\Machine Intelligence\Claude-IL2MissionUtility\references\Recordings\2026-10-03 20-46-21.mp4"
OUT = sys.argv[1]
FPS = float(sys.argv[2]) if len(sys.argv) > 2 else 1.0
SS = float(sys.argv[3]) if len(sys.argv) > 3 else 0
DUR = float(sys.argv[4]) if len(sys.argv) > 4 else 0
W, H = 1920, 1080

ocr = RapidOCR()
proc = subprocess.Popen([FF, "-v", "error", "-hwaccel", "cuda", "-ss", str(SS)] + (["-t", str(DUR)] if DUR else []) + ["-i", VIDEO, "-vf", f"fps={FPS}",
                         "-f", "rawvideo", "-pix_fmt", "bgr24", "-"],
                        stdout=subprocess.PIPE, bufsize=W * H * 3 * 2)
t0 = time.time()
with open(OUT, "w", encoding="utf-8") as f:
    i = 0
    while True:
        buf = proc.stdout.read(W * H * 3)
        if len(buf) < W * H * 3:
            break
        img = np.frombuffer(buf, np.uint8).reshape(H, W, 3)
        res, _ = ocr(img)
        items = []
        for box, text, conf in res or []:
            if float(conf) < 0.5:
                continue
            xs = [p[0] for p in box]; ys = [p[1] for p in box]
            items.append({"t": text, "c": round(float(conf), 2),
                          "x": int(min(xs)), "y": int(min(ys)),
                          "w": int(max(xs) - min(xs)), "h": int(max(ys) - min(ys))})
        f.write(json.dumps({"s": round(SS + i / FPS, 2), "items": items}, ensure_ascii=False) + "\n")
        f.flush()
        if i % 60 == 0:
            print(f"{SS + i / FPS:.0f}s done, {time.time() - t0:.0f}s elapsed", flush=True)
        i += 1
print("finished", i)

