from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence

import numpy as np
from PIL import Image, ImageDraw

from .model import Board, Piece


Rect = tuple[float, float, float, float]


def _cv2():
    try:
        import cv2  # type: ignore
    except ImportError as exc:
        raise RuntimeError("图像识别需要 OpenCV，请执行 pip install -r requirements.txt") from exc
    return cv2


def _pixel_rect(rect: Rect, width: int, height: int) -> tuple[int, int, int, int]:
    x, y, w, h = rect
    return round(x * width), round(y * height), round((x + w) * width), round((y + h) * height)


@dataclass(frozen=True, slots=True)
class VisionConfig:
    rows: int = 8
    cols: int = 8
    board_rect: Rect = (0.026, 0.259, 0.948, 0.440)
    tray_rects: tuple[Rect, ...] = (
        (0.055, 0.735, 0.245, 0.075),
        (0.315, 0.735, 0.365, 0.075),
        (0.700, 0.735, 0.245, 0.075),
    )
    cell_inner_ratio: float = 0.58
    occupied_delta: float = 35.0
    target_min_pixels: int = 45
    piece_cell_ratio: float = 0.073
    piece_size_tolerance: float = 0.40

    @classmethod
    def from_dict(cls, values: dict[str, Any]) -> "VisionConfig":
        data = dict(values)
        data["board_rect"] = tuple(data["board_rect"])
        data["tray_rects"] = tuple(tuple(item) for item in data["tray_rects"])
        return cls(**data)


@dataclass(frozen=True, slots=True)
class PieceObservation:
    piece: Piece
    # (局部行, 局部列, 截图 x, 截图 y)
    cell_centers: tuple[tuple[int, int, float, float], ...]
    tray_index: int

    def anchor(self) -> tuple[tuple[int, int], tuple[float, float]]:
        r, c, x, y = min(self.cell_centers, key=lambda item: (item[0], item[1]))
        return (r, c), (x, y)


@dataclass(frozen=True, slots=True)
class ScreenState:
    board: Board
    pieces: tuple[PieceObservation, ...]
    image_size: tuple[int, int]
    board_pixel_rect: tuple[int, int, int, int]
    occupancy_values: tuple[float, ...]

    def board_cell_center(self, row: int, col: int) -> tuple[float, float]:
        left, top, right, bottom = self.board_pixel_rect
        return (
            left + (col + 0.5) * (right - left) / self.board.cols,
            top + (row + 0.5) * (bottom - top) / self.board.rows,
        )


class VisionRecognizer:
    def __init__(self, config: VisionConfig | None = None) -> None:
        self.config = config or VisionConfig()

    def recognise(self, image: Image.Image | np.ndarray) -> ScreenState:
        cv2 = _cv2()
        rgb = np.asarray(image.convert("RGB") if isinstance(image, Image.Image) else image)
        height, width = rgb.shape[:2]
        board_rect = _pixel_rect(self.config.board_rect, width, height)
        left, top, right, bottom = board_rect
        board_rgb = rgb[top:bottom, left:right]
        board_hsv = cv2.cvtColor(board_rgb, cv2.COLOR_RGB2HSV)

        cell_values: list[float] = []
        patches: list[tuple[int, int, np.ndarray]] = []
        cell_h = board_rgb.shape[0] / self.config.rows
        cell_w = board_rgb.shape[1] / self.config.cols
        half_h = cell_h * self.config.cell_inner_ratio / 2
        half_w = cell_w * self.config.cell_inner_ratio / 2
        for row in range(self.config.rows):
            for col in range(self.config.cols):
                cy, cx = (row + 0.5) * cell_h, (col + 0.5) * cell_w
                y1, y2 = max(0, round(cy - half_h)), min(board_hsv.shape[0], round(cy + half_h))
                x1, x2 = max(0, round(cx - half_w)), min(board_hsv.shape[1], round(cx + half_w))
                patch = board_hsv[y1:y2, x1:x2]
                value = float(np.median(patch[:, :, 2]))
                cell_values.append(value)
                patches.append((row, col, patch))

        # 用一维双聚类区分深蓝空格与明亮方块；即使后期棋盘占用超过 75% 也能工作。
        values = np.asarray(cell_values, dtype=np.float32)
        low, high = float(values.min()), float(values.max())
        for _ in range(12):
            midpoint = (low + high) / 2
            low_group, high_group = values[values <= midpoint], values[values > midpoint]
            if not len(low_group) or not len(high_group):
                break
            next_low, next_high = float(low_group.mean()), float(high_group.mean())
            if abs(next_low - low) + abs(next_high - high) < 0.05:
                low, high = next_low, next_high
                break
            low, high = next_low, next_high
        threshold = (low + high) / 2 if high - low >= self.config.occupied_delta else low + self.config.occupied_delta
        matrix = [[0 for _ in range(self.config.cols)] for _ in range(self.config.rows)]
        targets: dict[tuple[int, int], str] = {}
        for index, (row, col, patch) in enumerate(patches):
            if cell_values[index] >= threshold:
                matrix[row][col] = 1
                tag = self._classify_target(patch)
                if tag:
                    targets[(row, col)] = tag

        observations: list[PieceObservation] = []
        for tray_index, rect in enumerate(self.config.tray_rects):
            observation = self._recognise_piece(rgb, rect, tray_index)
            if observation is not None:
                observations.append(observation)

        return ScreenState(
            board=Board.from_matrix(matrix, targets),
            pieces=tuple(observations),
            image_size=(width, height),
            board_pixel_rect=board_rect,
            occupancy_values=tuple(cell_values),
        )

    def _classify_target(self, hsv_patch: np.ndarray) -> str | None:
        # OpenCV H 范围为 0～179。先排除低饱和度的米色方块底板。
        h, s, v = (hsv_patch[:, :, i] for i in range(3))
        vivid = (s >= 105) & (v >= 110)
        ranges = {
            "red": ((h <= 8) | (h >= 174)) & vivid,
            "yellow": (h >= 17) & (h <= 38) & vivid,
            "green": (h >= 40) & (h <= 82) & vivid,
            "blue": (h >= 83) & (h <= 112) & vivid,
            "purple": (h >= 113) & (h <= 148) & vivid,
            "pink": (h >= 149) & (h <= 173) & vivid,
        }
        counts = {name: int(mask.sum()) for name, mask in ranges.items()}
        # 所有宝石都有金色描边。若直接取最大色块，蓝/绿/粉宝石可能被误判为黄色；
        # 因此先找足够明显的非黄色核心，仅在没有彩色核心时判为星星。
        non_yellow = {name: count for name, count in counts.items() if name != "yellow"}
        other_name, other_count = max(non_yellow.items(), key=lambda item: item[1])
        if other_count >= self.config.target_min_pixels:
            return other_name
        name, count = "yellow", counts["yellow"]
        return name if count >= self.config.target_min_pixels else None

    def _recognise_piece(self, rgb: np.ndarray, rect: Rect, tray_index: int) -> PieceObservation | None:
        cv2 = _cv2()
        height, width = rgb.shape[:2]
        left, top, right, bottom = _pixel_rect(rect, width, height)
        crop = rgb[top:bottom, left:right]
        if crop.size == 0:
            return None
        gray = cv2.cvtColor(crop, cv2.COLOR_RGB2GRAY)
        blurred = cv2.GaussianBlur(gray, (5, 5), 0)
        edges = cv2.Canny(blurred, 35, 110)
        edges = cv2.morphologyEx(edges, cv2.MORPH_CLOSE, np.ones((3, 3), np.uint8), iterations=1)
        contours, _ = cv2.findContours(edges, cv2.RETR_LIST, cv2.CHAIN_APPROX_SIMPLE)

        expected = width * self.config.piece_cell_ratio
        tolerance = self.config.piece_size_tolerance
        minimum, maximum = expected * (1 - tolerance), expected * (1 + tolerance)
        candidates: list[tuple[float, float, float]] = []
        for contour in contours:
            x, y, w, h = cv2.boundingRect(contour)
            if not (minimum <= w <= maximum and minimum <= h <= maximum):
                continue
            if not (0.72 <= w / h <= 1.38):
                continue
            area = abs(cv2.contourArea(contour))
            if area < w * h * 0.24:
                continue
            candidates.append((x + w / 2, y + h / 2, (w + h) / 2))

        # 同一方块可能产生内外两层轮廓，按中心距离去重，优先保留较大的轮廓。
        selected: list[tuple[float, float, float]] = []
        for candidate in sorted(candidates, key=lambda item: item[2], reverse=True):
            cx, cy, side = candidate
            if all((cx - x) ** 2 + (cy - y) ** 2 > (0.35 * expected) ** 2 for x, y, _ in selected):
                selected.append(candidate)
        if not selected:
            return None

        side = float(np.median([item[2] for item in selected]))
        min_x = min(item[0] for item in selected)
        min_y = min(item[1] for item in selected)
        grid_cells: dict[tuple[int, int], tuple[float, float]] = {}
        for cx, cy, _ in selected:
            row = round((cy - min_y) / side)
            col = round((cx - min_x) / side)
            grid_cells[(row, col)] = (left + cx, top + cy)

        crop_hsv = cv2.cvtColor(crop, cv2.COLOR_RGB2HSV)
        tags: list[tuple[int, int, str]] = []
        # 蛋图标中心是紫色，但外层米黄色方块才是稳定特征，采样范围需覆盖中心外圈。
        tag_radius = max(10, round(side * 0.36))
        for (row, col), (screen_x, screen_y) in grid_cells.items():
            cx, cy = round(screen_x - left), round(screen_y - top)
            patch = crop_hsv[
                max(0, cy - tag_radius):min(crop_hsv.shape[0], cy + tag_radius),
                max(0, cx - tag_radius):min(crop_hsv.shape[1], cx + tag_radius),
            ]
            median_h, median_s, median_v = np.median(patch.reshape(-1, 3), axis=0)
            if 12 <= median_h <= 35 and 80 <= median_s <= 145 and 165 <= median_v <= 238:
                tags.append((row, col, "egg"))

        piece = Piece(tuple(grid_cells), name=f"tray_{tray_index}", tags=tuple(tags))
        centers = tuple(sorted((r, c, *xy) for (r, c), xy in grid_cells.items()))
        return PieceObservation(piece, centers, tray_index)

    def draw_debug(self, image: Image.Image, state: ScreenState, output: str | Path) -> None:
        canvas = image.convert("RGB").copy()
        draw = ImageDraw.Draw(canvas)
        left, top, right, bottom = state.board_pixel_rect
        cell_w = (right - left) / state.board.cols
        cell_h = (bottom - top) / state.board.rows
        target_map = state.board.target_map()
        for row in range(state.board.rows):
            for col in range(state.board.cols):
                box = (
                    round(left + col * cell_w), round(top + row * cell_h),
                    round(left + (col + 1) * cell_w), round(top + (row + 1) * cell_h),
                )
                colour = "#ff4040" if state.board.is_occupied(row, col) else "#50e080"
                draw.rectangle(box, outline=colour, width=3)
                if (row, col) in target_map:
                    draw.text((box[0] + 5, box[1] + 5), target_map[(row, col)], fill="white")
        for observation in state.pieces:
            for r, c, x, y in observation.cell_centers:
                radius = 10
                draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill="#ff3030")
                draw.text((x + 12, y - 8), f"{observation.tray_index}:{r},{c}", fill="white")
        canvas.save(output)


def load_vision_config(path: str | Path) -> VisionConfig:
    with open(path, "r", encoding="utf-8") as handle:
        root = json.load(handle)
    return VisionConfig.from_dict(root.get("vision", root))
