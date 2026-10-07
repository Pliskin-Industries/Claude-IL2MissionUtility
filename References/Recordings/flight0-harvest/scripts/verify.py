"""Targeted 2x-upscaled OCR in short windows; writes verify.jsonl."""
import json, subprocess, sys, time
import numpy as np, cv2
from rapidocr_onnxruntime import RapidOCR

FF = r"C:\Users\BarronVonVuudmiester\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-9.0.2-full_build\bin\ffmpeg.exe"
VIDEO = r"C:\Machine Intelligence\Claude-IL2MissionUtility\references\Recordings\2026-10-03 20-46-21.mp4"
S = sys.argv[1]; OFF = 45.5
windows = json.load(open(f"{S}/windows.json"))   # list of [mission_start, mission_end]
W, H = 1920, 1080
ocr = RapidOCR()
out = open(f"{S}/verify.jsonl", "w", encoding="utf-8")
t0 = time.time()
for a, b in windows:
    va = a - OFF
    proc = subprocess.Popen([FF, "-v", "error", "-hwaccel", "cuda", "-ss", f"{va:.2f}", "-t", f"{b - a:.2f}",
                             "-i", VIDEO, "-vf", "fps=5", "-f", "rawvideo", "-pix_fmt", "bgr24", "-"],
                            stdout=subprocess.PIPE)
    i = 0
    while True:
        buf = proc.stdout.read(W * H * 3)
        if len(buf) < W * H * 3:
            break
        img = np.frombuffer(buf, np.uint8).reshape(H, W, 3)
        g = img[:, :, 1]
        big = cv2.resize(g, None, fx=2, fy=2, interpolation=cv2.INTER_CUBIC)
        res, _ = ocr(cv2.cvtColor(big, cv2.COLOR_GRAY2BGR))
        items = [{"t": t, "c": round(float(c), 2), "x": int(bx[0][0] / 2), "y": int(bx[0][1] / 2)}
                 for bx, t, c in (res or []) if float(c) >= 0.4]
        out.write(json.dumps({"s": round(va + i / 5, 2), "items": items}, ensure_ascii=False) + "\n")
        out.flush()
        i += 1
    print(f"window {a}-{b} done, {time.time() - t0:.0f}s", flush=True)
