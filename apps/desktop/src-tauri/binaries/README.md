# Sidecar binaries

Coloque aqui os binarios do `runtime-sidecar` para empacotamento Tauri.

- Nome base esperado: `runtime-sidecar`
- O Tauri adiciona sufixo/plataforma automaticamente durante o bundle.
- Em desenvolvimento local, o desktop usa `COMPANION_SIDECAR_BIN` para localizar o executavel.
- O `build.rs` do desktop prepara automaticamente `runtime-sidecar-<target-triple>` nesta pasta antes do passo do Tauri.
- Para gerar o artefato real de release (host target automatico): `powershell -ExecutionPolicy Bypass -File tools/scripts/prepare-sidecar.ps1`.
- Para gate de validacao do bundle e protocolo sidecar: `powershell -ExecutionPolicy Bypass -File tools/scripts/smoke-sidecar-bundle.ps1`.
