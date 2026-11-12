"""Validate first; publish repairs from staging without pip overwriting active DLLs."""
import ctypes
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import uuid

PACKAGES = ("nvidia-cublas-cu12==12.8.4.1", "nvidia-cuda-runtime-cu12==12.8.90", "nvidia-cudnn-cu12==9.7.1.26")
DLLS = ("nvidia/cublas/bin/cublas64_12.dll", "nvidia/cublas/bin/cublasLt64_12.dll",
        "nvidia/cuda_runtime/bin/cudart64_12.dll", "nvidia/cudnn/bin/cudnn64_9.dll")
BUSY = "GPU_RUNTIME_IN_USE"


def load(root):
    directories = [str(root / "nvidia" / part / "bin") for part in ("cublas", "cuda_runtime", "cudnn")]
    handles = [os.add_dll_directory(path) for path in directories]
    os.environ["PATH"] = os.pathsep.join(directories) + os.pathsep + os.environ.get("PATH", "")
    libraries = [ctypes.WinDLL(str(root / path)) for path in DLLS]
    return handles, libraries


def validate(root):
    # An isolated process must release all DLLs before a repair can start.
    result = subprocess.run([sys.executable, "-I", "-c", VALIDATOR, str(root)], capture_output=True, text=True)
    return result.returncode == 0


VALIDATOR = """import os,ctypes,sys
from pathlib import Path
root=Path(sys.argv[1])
directories=[str(root/'nvidia'/part/'bin') for part in ('cublas','cuda_runtime','cudnn')]
handles=[os.add_dll_directory(path) for path in directories]
os.environ['PATH']=os.pathsep.join(directories)+os.pathsep+os.environ.get('PATH','')
libraries=[ctypes.WinDLL(str(root/path)) for path in ('nvidia/cublas/bin/cublas64_12.dll','nvidia/cublas/bin/cublasLt64_12.dll','nvidia/cuda_runtime/bin/cudart64_12.dll','nvidia/cudnn/bin/cudnn64_9.dll')]
"""


def ensure_unoccupied(root):
    if os.name != "nt":
        return
    from ctypes import wintypes
    create = ctypes.windll.kernel32.CreateFileW
    create.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    create.restype = wintypes.HANDLE
    close = ctypes.windll.kernel32.CloseHandle
    close.argtypes = [wintypes.HANDLE]
    for path in root.rglob("*.dll") if root.exists() else ():
        handle = create(str(path), 0xC0000000, 0, None, 3, 0x80, None)
        if handle == ctypes.c_void_p(-1).value:
            raise RuntimeError(BUSY + ": " + str(path))
        close(handle)


def repair(root):
    root = root.absolute()
    if root.is_symlink() or getattr(root, "is_junction", lambda: False)():
        raise ValueError("Unexpected GPU runtime link")
    root = root.resolve()
    if root.name != "ai-gpu-runtime":
        raise ValueError("Unexpected GPU runtime directory")
    if root.is_symlink() or (root.exists() and any(path.is_symlink() or getattr(path, "is_junction", lambda: False)() for path in root.rglob("*"))):
        raise ValueError("GPU runtime contains unexpected links")
    if validate(root):
        print("GPU runtime already valid; installation skipped", flush=True)
        (root / "ready").write_text("\n".join(PACKAGES), encoding="utf-8")
        return
    ensure_unoccupied(root)
    (root / "ready").unlink(missing_ok=True)
    root.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ai-gpu-stage-", dir=root.parent) as temporary:
        staged = Path(temporary) / "ai-gpu-runtime"
        subprocess.run([sys.executable, "-I", "-u", "-m", "pip", "install", "--disable-pip-version-check", "--no-input", "--only-binary=:all:", "--target", str(staged), *PACKAGES], check=True)
        if not validate(staged):
            raise RuntimeError("GPU runtime staging validation failed")
        ensure_unoccupied(root)
        backup = root.with_name("ai-gpu-backup-" + uuid.uuid4().hex)
        existed = root.exists()
        if existed:
            root.rename(backup)
        try:
            staged.rename(root)
        except OSError:
            if existed:
                backup.rename(root)
            raise
        (root / "ready").write_text("\n".join(PACKAGES), encoding="utf-8")
        if existed:
            try:
                shutil.rmtree(backup)
            except OSError:
                print("Old GPU files retained at " + str(backup), flush=True)
    print("GPU runtime installation verified", flush=True)


if __name__ == "__main__":
    repair(Path(sys.argv[1]))
