"""Reyn Studio Python engine (sidecar).

Runs under the reyn-research venv (has torch + the research modules). The native
Rust app spawns this, reads `READY {port}` from stdout, connects over a localhost
TCP socket, and exchanges length-prefixed frames:

    frame  = u32 total_len, then `total_len` bytes = [u32 json_len][json][payload]

Requests carry only JSON (no payload); field responses carry the raw f32 field
in `payload` and its shape in the JSON. (Shared-memory transport is the planned
optimization; loopback TCP is the correct, robust first cut — a 32^3 field is ~400 KB.)
"""
import argparse
import json
import os
import socket
import struct
import sys
import traceback

import numpy as np


# -- framing -----------------------------------------------------------------
def _recvn(conn, n):
    buf = bytearray()
    while len(buf) < n:
        chunk = conn.recv(n - len(buf))
        if not chunk:
            raise ConnectionError("socket closed")
        buf += chunk
    return bytes(buf)


def recv(conn):
    (total,) = struct.unpack("<I", _recvn(conn, 4))
    body = _recvn(conn, total)
    (jl,) = struct.unpack("<I", body[:4])
    obj = json.loads(body[4:4 + jl].decode("utf-8"))
    return obj, body[4 + jl:]


def send(conn, obj, payload=b""):
    j = json.dumps(obj).encode("utf-8")
    body = struct.pack("<I", len(j)) + j + payload
    conn.sendall(struct.pack("<I", len(body)) + body)


# -- model / field ops -------------------------------------------------------
def list_checkpoints(research_dir):
    out = []
    for name in sorted(os.listdir(research_dir)):
        if name.endswith(".pth"):
            out.append(name)
    return out


class Engine:
    def __init__(self, research_dir):
        self.research_dir = research_dir
        sys.path.insert(0, research_dir)
        os.chdir(research_dir)
        import torch  # noqa: imported lazily so startup errors are reported cleanly
        self.torch = torch
        self.cache = {}

    def _load(self, path):
        if path in self.cache:
            return self.cache[path]
        torch = self.torch
        ck = torch.load(path, map_location="cpu", weights_only=False)
        cfg = ck["model_config"]
        is3d = any(k.endswith("weight") and ck["model_state_dict"][k].dim() == 5
                   for k in ck["model_state_dict"])
        if is3d:
            from models_3d import DirectFlowMap3D
            m = DirectFlowMap3D(**cfg)
        else:
            from time_moe_operator import DirectFlowMap
            m = DirectFlowMap(**cfg)
        m.load_state_dict(ck["model_state_dict"])
        m.eval()
        info = {"model": m, "cfg": cfg, "ta": ck["train_args"], "is3d": is3d,
                "scenario": ck["train_args"].get("scenario",
                    "obstacle" if cfg["in_channels"] > cfg["out_channels"] else "free")}
        self.cache[path] = info
        return info

    def predict_field(self, req):
        torch = self.torch
        path = req["model"]
        info = self._load(path)
        m, ta, is3d = info["model"], info["ta"], info["is3d"]
        scenario = info["scenario"]
        dt_frame = ta["dt"] * ta["stride"]
        horizon = int(req.get("steps", ta["max_steps"]))
        seed = int(req.get("seed", 1))

        if is3d:
            from dataset_3d import FlowDataset3D
            N = ta["grid_size"]
            ds = FlowDataset3D(scenario=scenario, num_trajectories=1,
                               trajectory_length=horizon + 1, N=N, solver_dt=ta["dt"],
                               nu=ta["nu"], warmup_steps=ta["warmup_steps"],
                               seq_len=horizon + 1, stride=ta["stride"], seed=seed + 50000)
            y0 = ds.trajectories[0][0:1]
            mask = ds.masks[0].unsqueeze(0)
            model_in = torch.cat([y0, mask], dim=1) if scenario == "obstacle" else y0
            with torch.no_grad():
                pred = m(model_in, torch.tensor([[horizon * dt_frame]]))
            field = pred[0].contiguous().numpy().astype(np.float32)  # [3, N, N, N]
        else:
            # 2D → return a [3, N, N] field (w-channel zero) so the client is uniform
            from obstacle_dataset import ObstacleFlowDataset
            N = ta["grid_size"]
            ds = ObstacleFlowDataset(num_trajectories=1, trajectory_length=horizon + 1,
                                     N=N, solver_dt=ta["dt"], warmup_steps=ta["warmup_steps"],
                                     seq_len=horizon + 1, stride=ta["stride"], seed=seed + 50000)
            y0 = ds.trajectories[0][0:1]
            mask = ds.masks[0].unsqueeze(0)
            with torch.no_grad():
                pred = m(torch.cat([y0, mask], dim=1), torch.tensor([[horizon * dt_frame]]))
            uv = pred[0].numpy().astype(np.float32)  # [2, N, N]
            field = np.concatenate([uv, np.zeros((1, N, N), np.float32)], 0)

        meta = {"ok": True, "shape": list(field.shape), "scenario": scenario,
                "dims": field.ndim - 1, "horizon": horizon}
        return field, meta


def serve(research_dir):
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", 0))
    srv.listen(1)
    port = srv.getsockname()[1]
    engine = Engine(research_dir)
    print("READY " + json.dumps({"port": port}), flush=True)
    # After the handshake we talk only over the socket. Silence stdout so library
    # prints (e.g. dataset generation) don't hit the closed pipe and SIGPIPE us.
    sys.stdout.flush()
    sys.stdout = open(os.devnull, "w")

    conn, _ = srv.accept()
    while True:
        try:
            obj, _ = recv(conn)
        except (ConnectionError, OSError):
            break
        op = obj.get("op")
        try:
            if op == "ping":
                send(conn, {"ok": True})
            elif op == "list_models":
                send(conn, {"ok": True, "models": list_checkpoints(research_dir)})
            elif op == "predict_field":
                field, meta = engine.predict_field(obj)
                send(conn, meta, field.tobytes())
            else:
                send(conn, {"ok": False, "error": f"unknown op: {op}"})
        except Exception as exc:  # never die on a request
            send(conn, {"ok": False, "error": f"{exc}", "trace": traceback.format_exc()[-800:]})


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--research-dir", required=True)
    serve(p.parse_args().research_dir)
