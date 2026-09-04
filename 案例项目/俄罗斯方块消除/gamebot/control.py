from __future__ import annotations

import ctypes
from ctypes import wintypes
import io
import subprocess
import time
from dataclasses import dataclass
from typing import Protocol

from PIL import Image, ImageGrab


@dataclass(frozen=True, slots=True)
class CapturedFrame:
    image: Image.Image
    # 截图左上角在实际屏幕/设备坐标中的位置。
    origin: tuple[int, int] = (0, 0)


class Controller(Protocol):
    def capture(self) -> CapturedFrame: ...
    def drag(self, start: tuple[float, float], end: tuple[float, float], duration_ms: int) -> None: ...


def _find_window_rect(title_fragment: str) -> tuple[int, int, int, int]:
    if not hasattr(ctypes, "windll"):
        raise RuntimeError("Windows 窗口控制只能在 Windows 上运行")
    user32 = ctypes.windll.user32
    try:
        user32.SetProcessDPIAware()
    except Exception:
        pass
    matches: list[tuple[int, str]] = []
    callback_type = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_void_p)

    def visit(hwnd: int, _lparam: int) -> bool:
        if not user32.IsWindowVisible(hwnd):
            return True
        length = user32.GetWindowTextLengthW(hwnd)
        buffer = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buffer, length + 1)
        title = buffer.value
        if title_fragment.lower() in title.lower():
            matches.append((hwnd, title))
        return True

    user32.EnumWindows(callback_type(visit), 0)
    if not matches:
        raise RuntimeError(f"找不到标题包含 {title_fragment!r} 的窗口")
    hwnd, _ = matches[0]
    rect = wintypes.RECT()
    if not user32.GetWindowRect(hwnd, ctypes.byref(rect)):
        raise RuntimeError("读取微信窗口位置失败")
    return rect.left, rect.top, rect.right, rect.bottom


class WindowsController:
    def __init__(self, window_title: str = "微信") -> None:
        self.window_title = window_title

    def capture(self) -> CapturedFrame:
        rect = _find_window_rect(self.window_title)
        return CapturedFrame(ImageGrab.grab(bbox=rect, all_screens=True).convert("RGB"), rect[:2])

    def drag(self, start: tuple[float, float], end: tuple[float, float], duration_ms: int = 450) -> None:
        try:
            import pyautogui
        except ImportError as exc:
            raise RuntimeError("桌面拖动需要 pyautogui，请执行 pip install -r requirements.txt") from exc
        frame = self.capture()
        ox, oy = frame.origin
        pyautogui.moveTo(ox + start[0], oy + start[1], duration=0.08)
        pyautogui.dragTo(ox + end[0], oy + end[1], duration=duration_ms / 1000, button="left")


class AdbController:
    def __init__(self, adb_path: str = "adb", serial: str | None = None) -> None:
        self.adb_path = adb_path
        self.serial = serial

    def _command(self, *args: str) -> list[str]:
        command = [self.adb_path]
        if self.serial:
            command.extend(["-s", self.serial])
        return command + list(args)

    def capture(self) -> CapturedFrame:
        result = subprocess.run(
            self._command("exec-out", "screencap", "-p"),
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return CapturedFrame(Image.open(io.BytesIO(result.stdout)).convert("RGB"))

    def drag(self, start: tuple[float, float], end: tuple[float, float], duration_ms: int = 450) -> None:
        command = self._command(
            "shell", "input", "swipe",
            str(round(start[0])), str(round(start[1])),
            str(round(end[0])), str(round(end[1])), str(duration_ms),
        )
        subprocess.run(command, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def wait_for_stable(
    controller: Controller,
    settle_seconds: float = 0.8,
    threshold: float = 2.0,
    attempts: int = 8,
) -> CapturedFrame:
    """等待动画基本停止；返回最后一帧。单次等待很短，失败时也不会无限卡住。"""
    import numpy as np

    time.sleep(settle_seconds)
    previous = controller.capture()
    for _ in range(attempts):
        time.sleep(0.18)
        current = controller.capture()
        a = np.asarray(previous.image.resize((160, 280)).convert("L"), dtype=np.int16)
        b = np.asarray(current.image.resize((160, 280)).convert("L"), dtype=np.int16)
        if float(np.mean(np.abs(a - b))) <= threshold:
            return current
        previous = current
    return previous
