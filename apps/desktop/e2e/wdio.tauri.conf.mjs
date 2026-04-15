import os from "node:os";
import path from "node:path";
import { existsSync } from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const desktopWorkspace = path.resolve(__dirname, "..");
const repoRoot = path.resolve(desktopWorkspace, "..", "..");
const isWindows = process.platform === "win32";
const tauriDriverFallback = path.join(
  os.homedir(),
  ".cargo",
  "bin",
  `tauri-driver${isWindows ? ".exe" : ""}`
);

let tauriDriverProcess;
let expectedExit = false;

function resolveTauriDriverPath() {
  const explicitPath = process.env.TAURI_DRIVER_PATH;
  if (explicitPath && existsSync(explicitPath)) {
    return explicitPath;
  }
  return tauriDriverFallback;
}

function resolveDesktopBinaryPath() {
  const explicitBinaryPath = process.env.COMPANION_TAURI_APP_PATH;
  if (explicitBinaryPath) {
    return explicitBinaryPath;
  }
  const binaryName = `companion-desktop${isWindows ? ".exe" : ""}`;
  const candidates = [
    path.join(repoRoot, "target", "debug", binaryName),
    path.join(desktopWorkspace, "src-tauri", "target", "debug", binaryName)
  ];
  const existingCandidate = candidates.find((candidate) => existsSync(candidate));
  return existingCandidate ?? candidates[0];
}

function resolveEdgeDriverPath() {
  const explicitPath = process.env.EDGE_WEBDRIVER_PATH;
  if (explicitPath && existsSync(explicitPath)) {
    return explicitPath;
  }
  if (!isWindows) {
    return null;
  }
  const whereResult = spawnSync("where", ["msedgedriver"], {
    cwd: repoRoot,
    encoding: "utf-8",
    shell: true
  });
  if (whereResult.status !== 0) {
    return null;
  }
  const firstLine = whereResult.stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length > 0);
  return firstLine ?? null;
}

function shutdownTauriDriver() {
  expectedExit = true;
  if (!tauriDriverProcess) {
    return;
  }
  tauriDriverProcess.kill();
  tauriDriverProcess = undefined;
}

function ensureBuildArtifacts() {
  const buildResult = spawnSync(
    "npm",
    ["run", "tauri:build", "--workspace", "@companion/desktop", "--", "--debug", "--no-bundle"],
    {
      cwd: repoRoot,
      stdio: "inherit",
      shell: true
    }
  );
  if (buildResult.status !== 0) {
    throw new Error("[tauri-e2e] falha ao gerar binario debug do desktop");
  }
}

function registerShutdownHooks() {
  const cleanup = () => {
    shutdownTauriDriver();
  };
  process.on("exit", () => {
    shutdownTauriDriver();
  });
  process.on("SIGINT", () => {
    cleanup();
    process.exit(130);
  });
  process.on("SIGTERM", () => {
    cleanup();
    process.exit(143);
  });
  process.on("SIGHUP", () => {
    cleanup();
    process.exit(129);
  });
  process.on("SIGBREAK", () => {
    cleanup();
    process.exit(131);
  });
}

registerShutdownHooks();

export const config = {
  host: "127.0.0.1",
  port: 4444,
  specs: [path.join(__dirname, "specs", "*.tauri.e2e.mjs")],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      "tauri:options": {
        application: resolveDesktopBinaryPath()
      }
    }
  ],
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: {
    ui: "bdd",
    timeout: 120000
  },
  onPrepare: () => {
    ensureBuildArtifacts();
    const desktopBinaryPath = resolveDesktopBinaryPath();
    if (!existsSync(desktopBinaryPath)) {
      throw new Error(`[tauri-e2e] binario desktop nao encontrado: ${desktopBinaryPath}`);
    }
  },
  beforeSession: () => {
    const tauriDriverPath = resolveTauriDriverPath();
    if (!existsSync(tauriDriverPath)) {
      throw new Error(
        `[tauri-e2e] tauri-driver nao encontrado. Instale com 'cargo install tauri-driver' ou configure TAURI_DRIVER_PATH. Caminho tentado: ${tauriDriverPath}`
      );
    }
    const edgeDriverPath = resolveEdgeDriverPath();
    const tauriDriverArgs = [];
    if (edgeDriverPath) {
      tauriDriverArgs.push("--native-driver", edgeDriverPath);
    } else if (isWindows) {
      throw new Error(
        "[tauri-e2e] msedgedriver nao encontrado. Instale com 'winget install --id Microsoft.EdgeDriver' ou configure EDGE_WEBDRIVER_PATH."
      );
    }

    tauriDriverProcess = spawn(tauriDriverPath, tauriDriverArgs, {
      stdio: [null, process.stdout, process.stderr]
    });
    tauriDriverProcess.on("error", (error) => {
      console.error("[tauri-e2e] erro no tauri-driver:", error);
      process.exit(1);
    });
    tauriDriverProcess.on("exit", (code) => {
      if (!expectedExit) {
        console.error("[tauri-e2e] tauri-driver encerrou antes do esperado. codigo:", code);
        process.exit(1);
      }
    });
  },
  afterSession: () => {
    shutdownTauriDriver();
  }
};
