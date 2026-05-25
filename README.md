# ai-gfx-dev

This repository contains a Rust-based DirectX graphics project for Windows development.

## Current status

The project currently creates a DX12 WARP device, reports its capabilities, and
includes tests that verify compute shaders can compile, be used to create
compute pipeline state objects, and execute through a TOML-driven compute graph
configuration with automatic dependency and barrier planning.

The repository vendors the official DirectXShaderCompiler x64 binaries under
`third_party/dxc/bin/x64` so build-time and runtime shader compilation use the
same checked-in DXC release.