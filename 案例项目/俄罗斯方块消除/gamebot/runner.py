from __future__ import annotations

import time
from dataclasses import dataclass

from .control import Controller, wait_for_stable
from .solver import Plan, Solver
from .vision import ScreenState, VisionRecognizer


@dataclass(frozen=True, slots=True)
class RunnerConfig:
    execute: bool = False
    drag_duration_ms: int = 450
    release_offset: tuple[float, float] = (0.0, 0.0)
    settle_seconds: float = 0.8
    stable_threshold: float = 2.0
    stable_attempts: int = 8
    max_steps: int | None = None


class GameRunner:
    def __init__(
        self,
        controller: Controller,
        recognizer: VisionRecognizer,
        solver: Solver,
        config: RunnerConfig | None = None,
    ) -> None:
        self.controller = controller
        self.recognizer = recognizer
        self.solver = solver
        self.config = config or RunnerConfig()

    def inspect_once(self) -> tuple[ScreenState, Plan]:
        frame = self.controller.capture()
        state = self.recognizer.recognise(frame.image)
        plan = self.solver.solve(state.board, [item.piece for item in state.pieces])
        return state, plan

    def run(self) -> None:
        step = 0
        previous_fingerprint: tuple | None = None
        last_observed_tray: tuple | None = None
        while self.config.max_steps is None or step < self.config.max_steps:
            frame = wait_for_stable(
                self.controller,
                self.config.settle_seconds,
                self.config.stable_threshold,
                self.config.stable_attempts,
            )
            state = self.recognizer.recognise(frame.image)
            if not state.pieces:
                raise RuntimeError("没有识别到托盘图形；请先用 inspect 和 debug.png 校准")
            fingerprint = (
                state.board.occupied,
                state.board.targets,
                tuple(item.piece.signature for item in state.pieces),
            )
            if previous_fingerprint is not None and fingerprint == previous_fingerprint:
                raise RuntimeError("拖动后画面状态没有变化，已停止以避免重复误操作；请校准 release_offset")
            if len(state.pieces) == 3:
                tray_signature = tuple(item.piece.signature for item in state.pieces)
                if tray_signature != last_observed_tray:
                    self.solver.distribution.observe(item.piece for item in state.pieces)
                    last_observed_tray = tray_signature
            plan = self.solver.solve(state.board, [item.piece for item in state.pieces])
            move = plan.first_move
            print(
                f"step={step} occupied={state.board.occupied_count} pieces={len(state.pieces)} "
                f"nodes={plan.explored_nodes} score={plan.score:.1f} plan={plan.moves}"
            )
            if move is None:
                print("当前没有合法落点，停止。")
                return
            if not self.config.execute:
                print("dry-run：未执行拖动。使用 --execute 后每轮只执行计划的第一步。")
                return

            previous_fingerprint = fingerprint
            observation = state.pieces[move.piece_index]
            (local_r, local_c), start = observation.anchor()
            end = state.board_cell_center(move.row + local_r, move.col + local_c)
            end = (end[0] + self.config.release_offset[0], end[1] + self.config.release_offset[1])
            self.controller.drag(start, end, self.config.drag_duration_ms)
            step += 1
            time.sleep(0.15)
