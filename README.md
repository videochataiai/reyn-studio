# Reyn Studio

Fully-native neural-CFD workbench. **Rust + egui + `wgpu`** (native Metal/Vulkan/DX12 —
*not* browser WebGPU), linked to the PyTorch models through a Python engine sidecar.

## Why this stack
- **Native `wgpu`** renders the 3D volumetric view directly on the GPU (Metal on macOS).
- **egui** is the fully-native immediate-mode UI (panels, sliders, metrics).
- **Python engine** keeps the PyTorch models where they belong; the native app talks to it
  over a control socket + shared memory (zero-copy for field arrays). Swappable later for a
  fully-native ONNX/ExecuTorch backend behind one `Engine` trait — zero UI change.

## Run
```bash
cargo run            # debug
cargo run --release  # smooth 3D
```

## Status
- [x] Native app shell — top bar, project rail, 3D-controls panel (matches the mockup)
- [ ] `wgpu` 3D volumetric viewport (flow field render)
- [ ] Python engine sidecar + IPC (inference on the real models)
