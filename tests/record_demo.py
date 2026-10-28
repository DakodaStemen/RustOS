#!/usr/bin/env python3
"""Record docs/demo.gif: boot RustOS in QEMU, type commands, capture real screendumps, stitch with ffmpeg."""
import os, socket, subprocess, sys, tempfile, time, shutil

IMG = "target/x86_64-unknown-none/release/bootimage-rust-os.bin"
KEYS = {" ": "spc", "\n": "ret"}
tmp = tempfile.mkdtemp(); sock = os.path.join(tmp, "m.sock")
q = subprocess.Popen(["qemu-system-x86_64", "-drive", f"format=raw,file={IMG}", "-display", "none",
                      "-monitor", f"unix:{sock},server,nowait", "-no-reboot"])
for _ in range(50):
    if os.path.exists(sock): break
    time.sleep(0.1)
m = socket.socket(socket.AF_UNIX); m.connect(sock); m.settimeout(1)
def cmd(c):
    m.sendall((c + "\n").encode())
    try: m.recv(4096)
    except socket.timeout: pass
n = 0
def snap():
    global n
    cmd(f"screendump {tmp}/f{n:04d}.ppm"); n += 1
def hold(sec):
    for _ in range(int(sec * 5)): snap(); time.sleep(0.2)
def type_line(s):
    for ch in s + "\n":
        cmd(f"sendkey {KEYS.get(ch, ch)}"); time.sleep(0.12); snap()
time.sleep(0.5); hold(2.5)
for line in ["help", "echo hello from rustos", "version", "ticks", "uptime"]:
    type_line(line); hold(1.2)
hold(1.5)
q.kill()
out = sys.argv[1] if len(sys.argv) > 1 else "docs/demo.gif"
subprocess.check_call(["ffmpeg", "-loglevel", "error", "-y", "-framerate", "8", "-i", f"{tmp}/f%04d.ppm",
    "-vf", "split[a][b];[a]palettegen[p];[b][p]paletteuse", out])
shutil.rmtree(tmp)
print("wrote", out, n, "frames")
