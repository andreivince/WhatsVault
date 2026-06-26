import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import {
  auditDemoAssetManifest,
  auditPublicRepository,
  auditTextContent,
  auditTrackedFileNames,
  hasPrivateDemoText,
} from "./public-hygiene.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../../..");

function privateMarker(parts) {
  return parts.join("");
}

const privateExportMarker = String.fromCharCode(70, 85, 76, 76, 32, 86, 65, 80, 79);
const privateExportToken = String.fromCharCode(86, 65, 80, 79);

describe("public repository hygiene guard", () => {
  it("blocks tracked private backup, database, plist, archive, and media artifacts by default", () => {
    const issues = auditTrackedFileNames([
      "fixtures/private-chat.sqlite",
      "fixtures/Manifest.db",
      "fixtures/Status.plist",
      "fixtures/chat-export.zip",
      "fixtures/photo.jpg",
      "fixtures/private-chat.png",
      "fixtures/_chat.txt",
      "docs/assets/whatsvault-readme-demo.mp4",
      "docs/assets/whatsvault-synthetic-demo.png",
      "apps/desktop/src-tauri/icons/icon.png",
      "apps/desktop/src-tauri/icons/private-chat.sqlite",
    ]);

    expect(issues.map((issue) => issue.path)).toEqual([
      "fixtures/private-chat.sqlite",
      "fixtures/Manifest.db",
      "fixtures/Status.plist",
      "fixtures/chat-export.zip",
      "fixtures/photo.jpg",
      "fixtures/private-chat.png",
      "fixtures/_chat.txt",
      "apps/desktop/src-tauri/icons/private-chat.sqlite",
    ]);
  });

  it("flags private local paths and personal roadmap evidence markers in text files", () => {
    const text = [
      privateMarker(["/Users/", "andrei", "vince", "/Downloads/export.zip"]),
      privateMarker(["C:/Users/", "andrei", "vince", "/Desktop/export.zip"]),
      privateMarker(["E", "B", "1", "/", "O", "-1"]),
      privateMarker(["Evidence", " Checklist"]),
      privateMarker(["star", " milestones"]),
      privateMarker(["download", " counts per release"]),
    ].join("\n");

    expect(auditTextContent("ROADMAP.md", text).map((issue) => issue.code)).toEqual([
      "private-macos-path",
      "private-windows-path",
      "personal-roadmap-evidence",
      "personal-roadmap-evidence",
      "personal-roadmap-metric",
      "personal-roadmap-metric",
      "personal-roadmap-section",
    ]);
  });

  it("flags personal launch-channel planning text", () => {
    const text = [
      privateMarker(["Hacker", " News"]),
      privateMarker(["R", "eddit"]),
      privateMarker(["Linked", "In"]),
    ].join("\n");

    expect(auditTextContent("ROADMAP.md", text).map((issue) => issue.code)).toEqual([
      "personal-roadmap-launch",
      "personal-roadmap-launch",
      "personal-roadmap-launch",
    ]);
  });

  it("allows placeholder paths and public privacy wording", () => {
    const text = [
      "/Users/example/Library/Application Support/MobileSync/Backup",
      "C:/Users/example/Apple/MobileSync/Backup",
      "/path/to/MobileSync/Backup",
      "Do not paste contact names or phone numbers.",
      "Use synthetic fixtures and aggregate counts.",
    ].join("\n");

    expect(auditTextContent("docs/example.md", text)).toEqual([]);
  });

  it("flags private-looking transcript content in text files", () => {
    const transcript = [
      privateMarker(["[6/23/26, 9:42:10 AM] +", "1", " (415) ", "555", "-2671: Please keep this private"]),
      privateMarker(["+", "55", " ", "11", " ", "91234", "-5678"]),
      "6/23/26, 9:44 AM - Jane Doe: Another exported message body",
    ].join("\n");

    expect(auditTextContent("fixtures/_chat.txt", transcript).map((issue) => issue.code)).toEqual([
      "private-phone-number",
      "private-whatsapp-transcript",
    ]);
  });

  it("flags standalone international phone numbers", () => {
    const phoneNumber = privateMarker(["+", "55", " ", "11", " ", "91234", "-5678"]);

    expect(auditTextContent("docs/debug-note.md", phoneNumber).map((issue) => issue.code)).toEqual([
      "private-phone-number",
    ]);
  });

  it("exposes one shared private demo text evaluator", () => {
    expect(hasPrivateDemoText("Synthetic demo copy only.")).toBe(false);
    expect(hasPrivateDemoText(privateMarker(["+", "55", " ", "11", " ", "91234", "-5678"]))).toBe(true);
    expect(hasPrivateDemoText("6/23/26, 9:44 AM - Jane Doe: Exported message body")).toBe(true);
  });

  it("verifies synthetic demo asset hashes from the public manifest", async () => {
    const tempRoot = await mkdtemp(resolve(tmpdir(), "whatsvault-demo-assets-"));
    try {
      const assetPath = "docs/assets/demo.mp4";
      const assetBytes = Buffer.from("synthetic demo");
      await mkdir(resolve(tempRoot, "docs/assets"), { recursive: true });
      await writeFile(resolve(tempRoot, assetPath), assetBytes);
      await writeFile(
        resolve(tempRoot, "docs/assets/demo-assets-manifest.json"),
        JSON.stringify({
          assets: [
            {
              path: assetPath,
              sha256: createHash("sha256").update(assetBytes).digest("hex"),
              source: "synthetic Playwright demo",
            },
          ],
        }),
      );

      await expect(auditDemoAssetManifest(tempRoot)).resolves.toEqual([]);
    } finally {
      await rm(tempRoot, { recursive: true, force: true });
    }
  });

  it("flags demo asset hash mismatches", async () => {
    const tempRoot = await mkdtemp(resolve(tmpdir(), "whatsvault-demo-assets-"));
    try {
      const assetPath = "docs/assets/demo.mp4";
      await mkdir(resolve(tempRoot, "docs/assets"), { recursive: true });
      await writeFile(resolve(tempRoot, assetPath), "private replacement");
      await writeFile(
        resolve(tempRoot, "docs/assets/demo-assets-manifest.json"),
        JSON.stringify({
          assets: [
            {
              path: assetPath,
              sha256: "0".repeat(64),
              source: "synthetic Playwright demo",
            },
          ],
        }),
      );

      await expect(auditDemoAssetManifest(tempRoot)).resolves.toMatchObject([
        {
          code: "demo-asset-hash-mismatch",
          path: assetPath,
        },
      ]);
    } finally {
      await rm(tempRoot, { recursive: true, force: true });
    }
  });

  it("keeps the hygiene implementation free of private literal markers", async () => {
    const implementationPaths = [
      "apps/desktop/scripts/privacy-rules.mjs",
      "apps/desktop/scripts/public-hygiene.test.mjs",
    ];
    const contents = await Promise.all(
      implementationPaths.map((implementationPath) => readFile(resolve(repoRoot, implementationPath), "utf8")),
    );

    expect(contents.some((content) => content.includes(privateExportMarker))).toBe(false);
    expect(contents.some((content) => content.includes(privateExportToken))).toBe(false);
  });

  it("keeps the current public candidate repository files free of private artifacts and personal roadmap text", async () => {
    await expect(auditPublicRepository(repoRoot)).resolves.toEqual([]);
  });
});
