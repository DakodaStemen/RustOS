#!/usr/bin/env python3
"""Boot RustOS in QEMU, type into the shell via the monitor, assert on serial output.

usage: tests/qemu_test.py [image]
"""
import os, socket, subprocess, sys, tempfile, time

IMG = sys.argv[1] if len(sys.argv) > 1 else "target/x86_64-unknown-none/release/bootimage-rust-os.bin"
KEYS = {" ": "spc", "\n": "ret"}


def main():
    tmp = tempfile.mkdtemp()
    sock, serial = os.path.join(tmp, "mon.sock"), os.path.join(tmp, "serial.log")
    q = subprocess.Popen(
        ["qemu-system-x86_64", "-drive", f"format=raw,file={IMG}", "-display", "none",
         "-serial", f"file:{serial}", "-monitor", f"unix:{sock},server,nowait", "-no-reboot"])
    try:
        for _ in range(50):
            if os.path.exists(sock):
                break
            time.sleep(0.1)
        m = socket.socket(socket.AF_UNIX); m.connect(sock); m.settimeout(2)

        def out():
            return open(serial, errors="replace").read() if os.path.exists(serial) else ""

        def wait(s, t=20):
            end = time.time() + t
            while time.time() < end:
                if s in out():
                    return
                time.sleep(0.2)
            sys.exit(f"FAIL: timed out waiting for {s!r}\n--- serial ---\n{out()}")

        def type_line(line):
            for ch in line + "\n":
                m.sendall(f"sendkey {KEYS.get(ch, ch)}\n".encode()); time.sleep(0.15)
                try: m.recv(4096)
                except socket.timeout: pass

        wait("RustOS booted"); print("ok: boot banner")
        wait("rustos> "); print("ok: prompt")
        type_line("echo hello"); wait("hello\n"); print("ok: echo")
        type_line("version"); wait("RustOS 0."); print("ok: version")
        time.sleep(1.5)
        type_line("ticks")
        wait("timer ticks: ")
        time.sleep(0.5)
        n = int(out().split("timer ticks: ")[-1].split()[0])
        assert n > 0, "timer never ticked"; print(f"ok: timer ticks ({n})")
        type_line("bogus"); wait("unknown command: bogus"); print("ok: unknown command")
        print("ALL PASSED")
    finally:
        q.kill()


main()
