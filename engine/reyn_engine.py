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
import math
import os
import socket
import struct
import sys
import traceback

os.environ.setdefault("PYTORCH_ENABLE_MPS_FALLBACK", "1")  # CPU-fallback any MPS gap

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


# -- pressure recovery (F-RecoverySettings) ----------------------------------
def _fd_poisson(rhs, dx, periodic, tol, max_iter):
    """Solve ∇²p = rhs on the FD 5-point Laplacian by conjugate gradient (A=−∇²
    is SPD, up to the constant null space for periodic BCs). Stops at relative
    residual `tol` or `max_iter`; returns (p, achieved_residual, iters)."""
    import torch
    dx2 = dx * dx

    def lap(p):
        if periodic:
            return (torch.roll(p, 1, 0) + torch.roll(p, -1, 0)
                    + torch.roll(p, 1, 1) + torch.roll(p, -1, 1) - 4.0 * p) / dx2
        pp = torch.nn.functional.pad(p[None, None], (1, 1, 1, 1))[0, 0]  # Dirichlet: p=0 outside
        return (pp[:-2, 1:-1] + pp[2:, 1:-1] + pp[1:-1, :-2] + pp[1:-1, 2:] - 4.0 * p) / dx2

    b = -rhs  # solve A p = b, A = −lap
    if periodic:
        b = b - b.mean()  # compatible RHS (remove the constant null direction)
    p = torch.zeros_like(rhs)
    r = b - (-lap(p))
    d = r.clone()
    rs = (r * r).sum()
    bn = b.norm() + 1e-12
    iters = 0
    for it in range(1, max_iter + 1):
        iters = it
        ad = -lap(d)
        denom = (d * ad).sum()
        if denom.abs() < 1e-30:
            break
        alpha = rs / denom
        p = p + alpha * d
        r = r - alpha * ad
        if periodic:
            p = p - p.mean()
        rs_new = (r * r).sum()
        if (rs_new.sqrt() / bn) < tol:
            break
        d = r + (rs_new / rs) * d
        rs = rs_new
    resid = float(((lap(p) - rhs).norm() / (rhs.norm() + 1e-12)).item())
    return p, resid, iters


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
        # inference on the GPU (Metal) when available — 5–10× faster forwards, so
        # TimeJump scrubbing stays responsive; solver data-gen stays on CPU.
        self.device = torch.device("mps") if torch.backends.mps.is_available() else torch.device("cpu")
        self.cache = {}
        self.traj2d = {}  # (model, seed) -> (y0, mask, trajectory, conditioned)

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
        m.to(self.device)
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
                pred = m(model_in.to(self.device), torch.tensor([[horizon * dt_frame]], device=self.device))
            field = pred[0].cpu().contiguous().numpy().astype(np.float32)  # [3, N, N, N]
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
                pred = m(torch.cat([y0, mask], dim=1).to(self.device),
                         torch.tensor([[horizon * dt_frame]], device=self.device))
            uv = pred[0].cpu().numpy().astype(np.float32)  # [2, N, N]
            field = np.concatenate([uv, np.zeros((1, N, N), np.float32)], 0)

        meta = {"ok": True, "shape": list(field.shape), "scenario": scenario,
                "dims": field.ndim - 1, "horizon": horizon}
        return field, meta

    def _traj2d(self, model, seed, need_len):
        """Cached solver trajectory for a 2D model, so TimeJump only re-runs the
        (fast) model forward pass per horizon instead of re-solving from scratch."""
        key = (model, seed)
        cached = self.traj2d.get(key)
        if cached is not None and cached[2].shape[0] >= need_len:
            return cached
        info = self._load(model)
        ta = info["ta"]
        N = ta["grid_size"]
        from obstacle_dataset import ObstacleFlowDataset
        length = max(need_len, 33)  # cover a full scrub after the first (slow) gen
        ds = ObstacleFlowDataset(num_trajectories=1, trajectory_length=length, N=N,
                                 solver_dt=ta["dt"], warmup_steps=ta["warmup_steps"],
                                 seq_len=length, stride=ta["stride"], seed=seed + 50000)
        conditioned = info["cfg"]["in_channels"] > info["cfg"]["out_channels"]
        out = (ds.trajectories[0][0:1], ds.masks[0].unsqueeze(0), ds.trajectories[0], conditioned)
        self.traj2d[key] = out
        return out

    def recover_pressure(self, field, method, tol, max_iter, periodic):
        """Recover pressure from a velocity field and report the Poisson-solve
        recovery error (relative residual). Spectral is an exact FFT inversion
        (residual ~ float32 eps); FD is an iterative CG solve to `tol`."""
        torch = self.torch
        from flow_quantities import _wavenumbers, pressure_from_velocity
        N = field.shape[-1]
        dx = 2.0 * math.pi / N
        kx, ky = _wavenumbers(N, field.device, field.dtype)
        u, v = field[:, 0:1], field[:, 1:2]
        uh, vh = torch.fft.fft2(u), torch.fft.fft2(v)
        dudx = torch.fft.ifft2(1j * kx * uh).real
        dudy = torch.fft.ifft2(1j * ky * uh).real
        dvdx = torch.fft.ifft2(1j * kx * vh).real
        dvdy = torch.fft.ifft2(1j * ky * vh).real
        adv_u = u * dudx + v * dudy
        adv_v = u * dvdx + v * dvdy
        div_adv = torch.fft.ifft2(1j * kx * torch.fft.fft2(adv_u)
                                  + 1j * ky * torch.fft.fft2(adv_v)).real
        rhs = (-div_adv)[0, 0]  # ∇²p = −div((u·∇)u)
        if method == "fd":
            return _fd_poisson(rhs, dx, periodic, float(tol), int(max_iter))
        p = pressure_from_velocity(field)  # spectral, exact
        lap = torch.fft.ifft2(-(kx ** 2 + ky ** 2) * torch.fft.fft2(p[None, None])).real[0, 0]
        resid = float((torch.norm(lap - rhs) / (torch.norm(rhs) + 1e-12)).item())
        return p, resid, 0

    def predict2d(self, req):
        """2D field for the pressure-recovery view: AI velocity + recovered
        pressure as `[3,N,N]` (u,v,p), a semigroup self-consistency number (Trust
        Meter), the pressure recovery residual, and — when `want_truth` — the
        solver truth `[3,N,N]` + RelL2/persist."""
        torch = self.torch
        info = self._load(req["model"])
        if info["is3d"]:
            raise ValueError("predict2d requires a 2D checkpoint")
        m, ta = info["model"], info["ta"]
        dt_frame = ta["dt"] * ta["stride"]
        horizon = int(req.get("steps", ta["max_steps"]))
        seed = int(req.get("seed", 1))
        want_truth = bool(req.get("want_truth", False))
        method = req.get("method", "spectral")
        tol = float(req.get("tolerance", 1e-5))
        max_iter = int(req.get("max_iter", 400))
        periodic = req.get("boundary", "periodic") != "dirichlet"
        from flow_quantities import pressure_from_velocity

        y0, mask, traj, conditioned = self._traj2d(req["model"], seed, horizon + 1)
        device = self.device
        mask_d = mask.to(device)

        def run(state, h):  # forward pass on the GPU
            s = state.to(device)
            model_in = torch.cat([s, mask_d], dim=1) if conditioned else s
            return m(model_in, torch.tensor([[h * dt_frame]], device=device))

        with torch.no_grad():
            pred_d = run(y0, horizon)  # [1,2,N,N] on device
            semi = None
            if horizon >= 2 and horizon % 2 == 0:  # semigroup: h vs (h/2 ∘ h/2)
                half = horizon // 2
                comp = run(run(y0, half), half)
                semi = float((torch.norm(comp - pred_d) / (torch.norm(pred_d) + 1e-9)).item())
            pred = pred_d.cpu()
            p_ai, p_resid, p_iters = self.recover_pressure(pred, method, tol, max_iter, periodic)

        N = pred.shape[-1]
        ai = torch.cat([pred[0], p_ai.unsqueeze(0)], 0).numpy().astype(np.float32)  # [3,N,N]
        meta = {"ok": True, "shape": [3, N, N], "scenario": info["scenario"], "dims": 2,
                "horizon": horizon, "dt_frame": dt_frame,
                "peak_p": float(p_ai.max()), "low_p": float(p_ai.min()), "semigroup": semi,
                "p_residual": p_resid, "p_iters": p_iters, "method": method}

        if want_truth and horizon < traj.shape[0]:
            truth = traj[horizon:horizon + 1]  # [1,2,N,N]
            with torch.no_grad():
                p_t = pressure_from_velocity(truth)
            truth3 = torch.cat([truth[0], p_t.unsqueeze(0)], 0).numpy().astype(np.float32)
            meta["rel_l2"] = float((torch.norm(pred - truth) / (torch.norm(truth) + 1e-9)).item())
            meta["persist"] = float((torch.norm(y0 - truth) / (torch.norm(truth) + 1e-9)).item())
            meta["has_truth"] = True
            return np.concatenate([ai, truth3], 0), meta  # [6,N,N]
        meta["has_truth"] = False
        return ai, meta


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
            elif op == "predict2d":
                field, meta = engine.predict2d(obj)
                send(conn, meta, field.tobytes())
            else:
                send(conn, {"ok": False, "error": f"unknown op: {op}"})
        except Exception as exc:  # never die on a request
            send(conn, {"ok": False, "error": f"{exc}", "trace": traceback.format_exc()[-800:]})


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--research-dir", required=True)
    serve(p.parse_args().research_dir)
