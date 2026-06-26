import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../../..");

async function readRepoFile(repoPath) {
  return readFile(join(repoRoot, repoPath), "utf8");
}

describe("public repository docs", () => {
  it("links the core public-maintenance files from README and contributing docs", async () => {
    const readme = await readRepoFile("README.md");
    const contributing = await readRepoFile("CONTRIBUTING.md");

    expect(readme).toContain("[CONTRIBUTING.md](CONTRIBUTING.md)");
    expect(readme).toContain("[SECURITY.md](SECURITY.md)");
    expect(readme).toContain("[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)");
    expect(readme).toContain("[CHANGELOG.md](CHANGELOG.md)");
    expect(contributing).toContain("[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)");
  });

  it("documents target-specific Tauri bundle roots for checksum generation", async () => {
    const ciRelease = await readRepoFile("docs/ci-release.md");

    expect(ciRelease).toContain("target/release/bundle");
    expect(ciRelease).toContain("target/<target-triple>/release/bundle");
    expect(ciRelease).toContain("WHATSVAULT_BUNDLE_DIR");
    expect(ciRelease).toContain("target/release/release-metadata");
  });

  it("configures dependency update coverage for the public repo", async () => {
    const dependabot = await readRepoFile(".github/dependabot.yml");
    const ciRelease = await readRepoFile("docs/ci-release.md");

    expect(dependabot).toContain("version: 2");
    expect(dependabot).toMatch(/package-ecosystem:\s+"cargo"[\s\S]*?directory:\s+"\/"/);
    expect(dependabot).toMatch(/package-ecosystem:\s+"npm"[\s\S]*?directory:\s+"\/apps\/desktop"/);
    expect(dependabot).toMatch(/package-ecosystem:\s+"github-actions"[\s\S]*?directory:\s+"\/"/);
    expect(ciRelease).toContain(".github/dependabot.yml");
  });

  it("keeps privacy-sensitive security reports on the private reporting path", async () => {
    const securityPolicy = await readRepoFile("SECURITY.md");

    expect(securityPolicy).toContain("GitHub private vulnerability reporting");
    expect(securityPolicy).toContain("Open a public issue only for sanitized, non-sensitive bugs");
    expect(securityPolicy).not.toContain("If that is not available");
  });

  it("keeps stable preflight docs aligned with the current blocker boundary", async () => {
    const contributing = await readRepoFile("CONTRIBUTING.md");
    const readme = await readRepoFile("README.md");

    expect(contributing).toContain("signing and notarization");
    expect(readme).toContain("macOS notarized signing and Windows code signing");
    expect(contributing).not.toContain("real-backup proof, packaged render smoke, or signing remain incomplete");
  });
});
