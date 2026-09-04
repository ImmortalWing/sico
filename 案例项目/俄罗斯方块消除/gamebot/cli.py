from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from PIL import Image

from .control import AdbController, WindowsController
from .model import Board
from .runner import GameRunner, RunnerConfig
from .shapes import PIECE_BY_NAME
from .solver import ScoreWeights, Solver, SolverConfig
from .vision import VisionConfig, VisionRecognizer


def _load_config(path: str | Path) -> dict[str, Any]:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def _solver(config: dict[str, Any]) -> Solver:
    values = dict(config.get("solver", {}))
    if "weights" in values:
        values["weights"] = ScoreWeights(**values["weights"])
    return Solver(SolverConfig(**values))


def _recognizer(config: dict[str, Any]) -> VisionRecognizer:
    return VisionRecognizer(VisionConfig.from_dict(config.get("vision", {})))


def _print_state(board: Board) -> None:
    targets = board.target_map()
    for row in range(board.rows):
        line = []
        for col in range(board.cols):
            if (row, col) in targets:
                line.append(targets[(row, col)][0].upper())
            else:
                line.append("#" if board.is_occupied(row, col) else ".")
        print(" ".join(line))


def command_demo(_args: argparse.Namespace) -> int:
    board = Board.from_matrix(
        (
            (0, 0, 0, 1, 1, 0, 0, 0),
            (1, 0, 0, 0, 0, 0, 0, 1),
            (0, 1, 0, 1, 1, 0, 1, 0),
            (0, 1, 0, 0, 0, 0, 1, 0),
            (0, 0, 0, 0, 0, 0, 0, 0),
            (1, 0, 0, 1, 1, 0, 0, 1),
            (0, 0, 0, 0, 0, 0, 0, 0),
            (0, 1, 1, 1, 1, 1, 1, 0),
        ),
        {(0, 3): "yellow", (2, 3): "blue", (5, 3): "purple"},
    )
    tray = (PIECE_BY_NAME["l3_a"], PIECE_BY_NAME["h4"], PIECE_BY_NAME["l3_b"])
    solver = Solver()
    plan = solver.solve(board, tray)
    print("初始棋盘：")
    _print_state(board)
    print(f"\n搜索节点：{plan.explored_nodes}，评分：{plan.score:.2f}")
    print("计划：", plan.moves)
    print("最终棋盘：")
    _print_state(plan.final_board)
    return 0


def command_inspect(args: argparse.Namespace) -> int:
    config = _load_config(args.config)
    recognizer = _recognizer(config)
    image = Image.open(args.image).convert("RGB")
    state = recognizer.recognise(image)
    print(f"图片：{state.image_size[0]}×{state.image_size[1]}")
    print(f"棋盘矩形：{state.board_pixel_rect}，占用格：{state.board.occupied_count}")
    _print_state(state.board)
    print("识别图形：")
    for index, observation in enumerate(state.pieces):
        print(
            f"  solver_index={index} tray={observation.tray_index} "
            f"cells={observation.piece.cells} tags={observation.piece.tags}"
        )
    solver = _solver(config)
    plan = solver.solve(state.board, [item.piece for item in state.pieces])
    print(f"计划：{plan.moves}，评分={plan.score:.2f}，搜索节点={plan.explored_nodes}")
    if args.debug:
        recognizer.draw_debug(image, state, args.debug)
        print(f"调试图已写入：{Path(args.debug).resolve()}")
    return 0


def command_run(args: argparse.Namespace) -> int:
    config = _load_config(args.config)
    control = config.get("control", {})
    controller = _controller(args.mode, control)
    runner_config = RunnerConfig(
        execute=args.execute,
        drag_duration_ms=control.get("drag_duration_ms", 450),
        release_offset=tuple(control.get("release_offset", (0, 0))),
        settle_seconds=control.get("settle_seconds", 0.8),
        stable_threshold=control.get("stable_threshold", 2.0),
        stable_attempts=control.get("stable_attempts", 8),
        max_steps=args.max_steps,
    )
    GameRunner(controller, _recognizer(config), _solver(config), runner_config).run()
    return 0


def _controller(mode: str, control: dict[str, Any]):
    if mode == "windows":
        return WindowsController(control.get("window_title", "微信"))
    return AdbController(control.get("adb_path", "adb"), control.get("adb_serial"))


def command_capture(args: argparse.Namespace) -> int:
    config = _load_config(args.config)
    controller = _controller(args.mode, config.get("control", {}))
    frame = controller.capture()
    frame.image.save(args.output)
    print(f"截图已写入：{Path(args.output).resolve()}，尺寸={frame.image.size}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="微信方块消除识别与规划")
    commands = parser.add_subparsers(dest="command", required=True)
    demo = commands.add_parser("demo", help="运行纯算法演示")
    demo.set_defaults(func=command_demo)

    inspect = commands.add_parser("inspect", help="离线识别一张截图")
    inspect.add_argument("image")
    inspect.add_argument("--config", default="config.json")
    inspect.add_argument("--debug")
    inspect.set_defaults(func=command_inspect)

    run = commands.add_parser("run", help="从微信窗口或 ADB 读取游戏")
    run.add_argument("--mode", choices=("windows", "adb"), default="windows")
    run.add_argument("--config", default="config.json")
    run.add_argument("--execute", action="store_true", help="真正执行拖动；默认只规划一次")
    run.add_argument("--max-steps", type=int)
    run.set_defaults(func=command_run)

    capture = commands.add_parser("capture", help="只截取微信窗口或设备画面，用于校准")
    capture.add_argument("--mode", choices=("windows", "adb"), default="windows")
    capture.add_argument("--config", default="config.json")
    capture.add_argument("--output", default="capture.png")
    capture.set_defaults(func=command_capture)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
