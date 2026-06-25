import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import {
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
      privateMarker(["FULL", " VAPO"]),
    ].join("\n");

    expect(auditTextContent("ROADMAP.md", text).map((issue) => issue.code)).toEqual([
      "private-macos-path",
      "private-windows-path",
      "private-export-name",
      "personal-roadmap-evidence",
      "personal-roadmap-evidence",
      "personal-roadmap-metric",
      "personal-roadmap-metric",
      "personal-roadmap-section",
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

  it("keeps the current public candidate repository files free of private artifacts and personal roadmap text", async () => {
    await expect(auditPublicRepository(repoRoot)).resolves.toEqual([]);
  });
});
