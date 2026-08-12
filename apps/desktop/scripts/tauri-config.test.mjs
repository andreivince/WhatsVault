import { access, readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const tauriDir = join(appDir, "src-tauri");
const configPath = join(tauriDir, "tauri.conf.json");
const capabilitiesPath = join(tauriDir, "capabilities", "default.json");
const backendLibraryPath = join(tauriDir, "src", "lib.rs");

async function readConfig() {
  return JSON.parse(await readFile(configPath, "utf8"));
}

async function readDefaultCapability() {
  return JSON.parse(await readFile(capabilitiesPath, "utf8"));
}

describe("Tauri release configuration", () => {
  it("declares an explicit visible, focused main desktop window", async () => {
    const config = await readConfig();
    const mainWindow = config.app?.windows?.[0];

    expect(mainWindow).toMatchObject({
      label: "main",
      title: "WhatsVault",
      url: "index.html",
      visible: true,
      focus: true,
      center: true,
      resizable: true,
      decorations: false,
    });
  });

  it("declares macOS, Windows, and PNG app icons that exist on disk", async () => {
    const config = await readConfig();
    const icons = config.bundle?.icon;

    expect(icons).toEqual([
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico",
    ]);

    await Promise.all(icons.map((iconPath) => access(join(tauriDir, iconPath))));
  });

  it("keeps macOS signing disabled until Developer ID signing is configured", async () => {
    const config = await readConfig();

    expect(config.bundle?.macOS?.signingIdentity).toBeUndefined();
    expect(config.bundle?.macOS?.entitlements).toBeUndefined();
  });

  it("keeps a restrictive production content security policy", async () => {
    const config = await readConfig();
    const csp = config.app?.security?.csp;

    expect(csp).toEqual({
      "default-src": "'self' customprotocol: asset:",
      "script-src": "'self' customprotocol: asset:",
      "style-src": "'self' customprotocol: asset:",
      "img-src": "'self' asset: http://asset.localhost data:",
      "media-src": "'self' asset: http://asset.localhost data:",
      "font-src": "'self'",
      "connect-src": "ipc: http://ipc.localhost",
      "object-src": "'none'",
      "base-uri": "'none'",
      "frame-ancestors": "'none'",
    });
  });

  it("keeps frontend plugin permissions minimal because file dialogs are backend-owned", async () => {
    const capability = await readDefaultCapability();

    expect(capability).toMatchObject({
      identifier: "default",
      windows: ["main"],
    });
    expect(capability.permissions).toEqual([
      "core:default",
      "core:window:allow-start-dragging",
      "core:window:allow-minimize",
      "core:window:allow-toggle-maximize",
      "core:window:allow-close",
    ]);
    expect(capability.permissions).not.toContain("dialog:default");
    expect(capability.permissions).not.toContain("dialog:allow-open");
    expect(capability.permissions).not.toContain("dialog:allow-save");
  });

  it("keeps a frameless window movable with only real custom window controls", async () => {
    const config = await readConfig();
    const capability = await readDefaultCapability();

    expect(config.app?.windows?.[0]?.decorations).toBe(false);
    expect(capability.permissions).toContain("core:window:allow-start-dragging");
    expect(capability.permissions).toContain("core:window:allow-close");
    expect(capability.permissions).toContain("core:window:allow-minimize");
    expect(capability.permissions).toContain("core:window:allow-toggle-maximize");
    expect(capability.permissions).not.toContain("core:window:allow-maximize");
  });

  it("keeps every desktop command asynchronous so local I/O cannot run on the main thread", async () => {
    const backendLibrary = await readFile(backendLibraryPath, "utf8");
    const commandDeclarations = [
      ...backendLibrary.matchAll(/#\[tauri::command\]\s+(async\s+)?fn\s+(\w+)/g),
    ];
    const synchronousCommands = commandDeclarations
      .filter(([, asyncKeyword]) => !asyncKeyword)
      .map(([, , commandName]) => commandName);

    expect(commandDeclarations.length).toBeGreaterThan(0);
    expect(synchronousCommands).toEqual([]);
    expect(backendLibrary).toContain("tauri::async_runtime::spawn_blocking");
  });
});
