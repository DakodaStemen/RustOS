#!/usr/bin/env python3
"""Boot RustOS in QEMU, type into the shell via the monitor, assert on serial output.

usage: tests/qemu_test.py [image]

Boots the image three times: interactive shell checks, `reboot`, `shutdown`.
(The in-kernel #[test_case] suite is separate: run `cargo test`.)
"""
import os, socket, subprocess, sys, tempfile, time

IMG = sys.argv[1] if len(sys.argv) > 1 else "target/x86_64-unknown-none/release/bootimage-rust-os.bin"
KEYS = {" ": "spc", "\n": "ret"}


class Vm:
    def __init__(self):
        tmp = tempfile.mkdtemp()
        sock, self.serial = os.path.join(tmp, "mon.sock"), os.path.join(tmp, "serial.log")
        self.q = subprocess.Popen(
            ["qemu-system-x86_64", "-drive", f"format=raw,file={IMG}", "-display", "none",
             "-serial", f"file:{self.serial}", "-monitor", f"unix:{sock},server,nowait", "-no-reboot"])
        for _ in range(50):
            if os.path.exists(sock):
                break
            time.sleep(0.1)
        self.m = socket.socket(socket.AF_UNIX); self.m.connect(sock); self.m.settimeout(2)

    def out(self):
        return open(self.serial, errors="replace").read() if os.path.exists(self.serial) else ""

    def wait(self, s, t=20):
        end = time.time() + t
        while time.time() < end:
            if s in self.out():
                return
            time.sleep(0.2)
        sys.exit(f"FAIL: timed out waiting for {s!r}\n--- serial ---\n{self.out()}")

    def key(self, k):
        self.m.sendall(f"sendkey {k}\n".encode()); time.sleep(0.15)
        try: self.m.recv(4096)
        except socket.timeout: pass

    def type_line(self, line):
        for ch in line + "\n":
            self.key(KEYS.get(ch, ch))

    def wait_exit(self, t=15):
        try:
            self.q.wait(timeout=t)
        except subprocess.TimeoutExpired:
            sys.exit(f"FAIL: QEMU did not exit\n--- serial ---\n{self.out()}")

    def kill(self):
        self.q.kill()


def shell_checks():
    vm = Vm()
    try:
        vm.wait("RustOS booted"); print("ok: boot banner")
        vm.wait("rustos> "); print("ok: prompt")
        vm.type_line("echo hello"); vm.wait("hello\n"); print("ok: echo")
        vm.type_line("version"); vm.wait("RustOS 0."); print("ok: version")
        time.sleep(1.5)
        vm.type_line("ticks")
        vm.wait("timer ticks: ")
        time.sleep(0.5)
        n = int(vm.out().split("timer ticks: ")[-1].split()[0])
        assert n > 0, "timer never ticked"; print(f"ok: timer ticks ({n})")
        vm.type_line("bogus"); vm.wait("unknown command: bogus"); print("ok: unknown command")

        vm.type_line("mem"); vm.wait("heap: ")
        vm.wait("total"); out = vm.out()
        total = int(out.split("heap: ")[-1].split("free, ")[1].split(" KiB")[0])
        assert total >= 1024, f"heap total {total} KiB < 1 MiB"; print(f"ok: mem (heap {total} KiB)")

        # history: Up recalls the previous command, Down walks forward to a blank line.
        vm.type_line("echo histtest")
        vm.wait("histtest\n")
        before = vm.out().count("histtest")
        vm.key("up"); vm.key("ret")
        end = time.time() + 10
        while time.time() < end and vm.out().count("histtest") < before + 2:  # redraw + output
            time.sleep(0.2)
        assert vm.out().count("histtest") >= before + 2, "Up did not recall previous command"
        print("ok: history up")
        # Down: after two Ups, Down moves one entry newer.
        vm.type_line("echo qq1"); vm.wait("qq1\n")
        vm.type_line("echo qq2"); vm.wait("qq2\n")
        vm.key("up"); vm.key("up"); vm.key("down"); vm.key("ret")
        end = time.time() + 10
        while time.time() < end and vm.out().count("\nqq2\n") < 2:
            time.sleep(0.2)
        assert vm.out().count("\nqq2\n") == 2 and vm.out().count("\nqq1\n") == 1, "Down did not move to newer entry"
        print("ok: history down")
    finally:
        vm.kill()


def power_check(cmd, banner):
    vm = Vm()
    try:
        vm.wait("rustos> ")
        vm.type_line(cmd); vm.wait(banner)
        vm.wait_exit()  # -no-reboot: reset/poweroff both end the QEMU process
        print(f"ok: {cmd} (QEMU exited)")
    finally:
        vm.kill()


shell_checks()
power_check("reboot", "rebooting...")
power_check("shutdown", "shutting down...")
print("ALL PASSED")
