import { basename, extname } from "node:path";

export const PRIVATE_DATA_EXTENSIONS = new Set([
  ".aac",
  ".db",
  ".gif",
  ".heic",
  ".jpeg",
  ".jpg",
  ".m4a",
  ".mov",
  ".mp4",
  ".opus",
  ".pdf",
  ".plist",
  ".png",
  ".sqlite",
  ".sqlite3",
  ".vcf",
  ".wav",
  ".webp",
  ".zip",
]);

export const PRIVATE_DATA_SUFFIXES = [
  ".db-shm",
  ".db-wal",
  ".sqlite-shm",
  ".sqlite-wal",
];

export const PRIVATE_DATA_FILE_NAMES = new Set(["_chat.txt"]);

export const PRIVATE_DATA_BASENAME_PATTERNS = [
  /^whatsapp chat\b.*\.txt$/i,
];

export const ALLOWED_PUBLIC_BINARY_FILES = new Set([
  "apps/desktop/src-tauri/icons/32x32.png",
  "apps/desktop/src-tauri/icons/64x64.png",
  "apps/desktop/src-tauri/icons/128x128.png",
  "apps/desktop/src-tauri/icons/128x128@2x.png",
  "apps/desktop/src-tauri/icons/Square30x30Logo.png",
  "apps/desktop/src-tauri/icons/Square44x44Logo.png",
  "apps/desktop/src-tauri/icons/Square71x71Logo.png",
  "apps/desktop/src-tauri/icons/Square89x89Logo.png",
  "apps/desktop/src-tauri/icons/Square107x107Logo.png",
  "apps/desktop/src-tauri/icons/Square142x142Logo.png",
  "apps/desktop/src-tauri/icons/Square150x150Logo.png",
  "apps/desktop/src-tauri/icons/Square284x284Logo.png",
  "apps/desktop/src-tauri/icons/Square310x310Logo.png",
  "apps/desktop/src-tauri/icons/StoreLogo.png",
  "apps/desktop/src-tauri/icons/icon.icns",
  "apps/desktop/src-tauri/icons/icon.ico",
  "apps/desktop/src-tauri/icons/icon.png",
  "docs/assets/whatsvault-readme-demo.mp4",
  "docs/assets/whatsvault-synthetic-demo.png",
]);

export const TEXT_FILE_EXTENSIONS = new Set([
  ".css",
  ".html",
  ".json",
  ".md",
  ".mjs",
  ".rs",
  ".srt",
  ".svg",
  ".toml",
  ".ts",
  ".tsx",
  ".txt",
  ".yaml",
  ".yml",
]);

export const TEXT_FILE_NAMES = new Set([
  ".gitignore",
  "AGENTS.md",
  "CONTRIBUTING.md",
  "LICENSE",
  "README.md",
  "SECURITY.md",
]);

const PLACEHOLDER_USER_NAMES = new Set(["example", "runner", "you"]);

function privateMarkerPattern(parts, flags = "i") {
  return new RegExp(parts.join(""), flags);
}

const TEXT_RULES = [
  {
    code: "private-macos-path",
    pattern: /\/Users\/([A-Za-z0-9._-]+)/g,
    message: "contains a non-placeholder macOS local path",
    demoSafety: true,
    shouldFlag: (match) => !PLACEHOLDER_USER_NAMES.has(match[1].toLowerCase()),
  },
  {
    code: "private-windows-path",
    pattern: /[A-Z]:[\\/]+Users[\\/]+([A-Za-z0-9._-]+)/gi,
    message: "contains a non-placeholder Windows local path",
    demoSafety: true,
    shouldFlag: (match) => !PLACEHOLDER_USER_NAMES.has(match[1].toLowerCase()),
  },
  {
    code: "private-email",
    pattern: privateMarkerPattern(["andrei", "@"]),
    message: "contains a private email marker",
    demoSafety: true,
  },
  {
    code: "private-phone-number",
    pattern: /\+\d{1,3}[\s.-]+(?:\d{1,4}[\s.-]+){1,3}\d{4,5}\b|(?:\(\d{3}\)|\d{3})[\s.-]+\d{3}[\s.-]+\d{4}\b/i,
    message: "contains a private-looking phone number",
    demoSafety: true,
  },
  {
    code: "private-whatsapp-transcript",
    pattern: /(?:^|\n)\s*(?:\[\d{1,2}\/\d{1,2}\/\d{2,4},?\s+[^\]]+\]\s+[^:\n]{1,80}:|\d{1,2}\/\d{1,2}\/\d{2,4},?\s+[^-\n]{1,40}\s+-\s+[^:\n]{1,80}:)/i,
    message: "contains private-looking WhatsApp transcript text",
    demoSafety: true,
  },
  {
    code: "personal-roadmap-evidence",
    pattern: privateMarkerPattern(["\\bE", "B", "1", "A?\\b"]),
    message: "contains personal evidence-roadmap language",
  },
  {
    code: "personal-roadmap-evidence",
    pattern: privateMarkerPattern(["\\bO", "-", "1", "A?\\b"]),
    message: "contains personal evidence-roadmap language",
  },
  {
    code: "personal-roadmap-metric",
    pattern: /star\s+milestones/i,
    message: "contains personal traction-tracking roadmap language",
  },
  {
    code: "personal-roadmap-metric",
    pattern: /fork\s+count/i,
    message: "contains personal traction-tracking roadmap language",
  },
  {
    code: "personal-roadmap-metric",
    pattern: /download\s+counts\s+per\s+release/i,
    message: "contains personal traction-tracking roadmap language",
  },
  {
    code: "personal-roadmap-metric",
    pattern: /traffic\s+spikes/i,
    message: "contains personal traction-tracking roadmap language",
  },
  {
    code: "personal-roadmap-metric",
    pattern: privateMarkerPattern(["maintainer\\s+or\\s+user\\s+test", "imonials"]),
    message: "contains personal traction-tracking roadmap language",
  },
  {
    code: "personal-roadmap-metric",
    pattern: /downstream\s+project\s+or\s+person\s+relying/i,
    message: "contains personal traction-tracking roadmap language",
  },
  {
    code: "personal-roadmap-section",
    pattern: /Evidence\s+Checklist/i,
    message: "contains personal evidence-roadmap language",
  },
  {
    code: "personal-roadmap-launch",
    pattern: privateMarkerPattern(["Hacker", "\\s+News"]),
    message: "contains personal launch-channel planning language",
  },
  {
    code: "personal-roadmap-launch",
    pattern: privateMarkerPattern(["R", "eddit"]),
    message: "contains personal launch-channel planning language",
  },
  {
    code: "personal-roadmap-launch",
    pattern: privateMarkerPattern(["Linked", "In"]),
    message: "contains personal launch-channel planning language",
  },
];

export function normalizeRepoPath(repoPath) {
  return repoPath.replaceAll("\\", "/");
}

export function isAllowedPublicBinary(repoPath) {
  return ALLOWED_PUBLIC_BINARY_FILES.has(normalizeRepoPath(repoPath));
}

export function isPrivateDataFile(repoPath) {
  const normalized = normalizeRepoPath(repoPath).toLowerCase();
  const fileName = basename(normalized);
  const extension = extname(normalized);
  return (
    PRIVATE_DATA_EXTENSIONS.has(extension) ||
    PRIVATE_DATA_SUFFIXES.some((suffix) => normalized.endsWith(suffix)) ||
    PRIVATE_DATA_FILE_NAMES.has(fileName) ||
    PRIVATE_DATA_BASENAME_PATTERNS.some((pattern) => pattern.test(fileName))
  );
}

export function isTextFile(repoPath) {
  return TEXT_FILE_EXTENSIONS.has(extname(repoPath).toLowerCase()) || TEXT_FILE_NAMES.has(basename(repoPath));
}

export function auditDemoText(content) {
  return auditTextContent("demo", content).filter((issue) =>
    TEXT_RULES.some((rule) => rule.demoSafety && rule.code === issue.code)
  );
}

export function hasPrivateDemoText(content) {
  return auditDemoText(content).length > 0;
}

export function auditTrackedFileNames(repoPaths) {
  return repoPaths
    .map(normalizeRepoPath)
    .filter((repoPath) => isPrivateDataFile(repoPath) && !isAllowedPublicBinary(repoPath))
    .map((repoPath) => ({
      code: "private-data-file",
      path: repoPath,
      message: "tracked file looks like a private chat, backup, database, plist, transcript, or media artifact",
    }));
}

export function auditTextContent(repoPath, content) {
  const issues = [];
  const normalizedPath = normalizeRepoPath(repoPath);

  for (const rule of TEXT_RULES) {
    const flags = rule.pattern.flags.includes("g") ? rule.pattern.flags : `${rule.pattern.flags}g`;
    const pattern = new RegExp(rule.pattern.source, flags);

    for (const match of content.matchAll(pattern)) {
      if (rule.shouldFlag && !rule.shouldFlag(match)) {
        continue;
      }
      issues.push({
        code: rule.code,
        path: normalizedPath,
        message: rule.message,
      });
      break;
    }
  }

  return issues;
}
