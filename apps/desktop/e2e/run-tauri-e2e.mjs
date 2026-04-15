import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..", "..", "..");

if (process.env.COMPANION_RUN_TAURI_E2E !== "1") {
  console.log(
    "[tauri-e2e] skip: defina COMPANION_RUN_TAURI_E2E=1 para executar o smoke E2E real."
  );
  process.exit(0);
}

const wdioCliScript = path.join(
  repoRoot,
  "node_modules",
  "@wdio",
  "cli",
  "bin",
  "wdio.js"
);

if (!existsSync(wdioCliScript)) {
  console.error(`[tauri-e2e] script wdio nao encontrado: ${wdioCliScript}`);
  process.exit(1);
}

const run = spawnSync(
  process.execPath,
  [wdioCliScript, "run", path.join(__dirname, "wdio.tauri.conf.mjs")],
  {
    cwd: __dirname,
    stdio: "inherit"
  }
);

process.exit(run.status ?? 1);
