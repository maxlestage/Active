import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";

// React's Server Components build has no hooks: importing the package there must still work.
describe.skipIf(!existsSync("dist/index.js"))("react-server condition", () => {
  it("imports Seo and createSeo without crashing", () => {
    const code = `import("./dist/index.js").then((m) => console.log(Object.keys(m).sort().join(",")))`;
    const out = execFileSync(process.execPath, ["--conditions=react-server", "--input-type=module", "-e", code]).toString();
    expect(out.trim()).toBe("Seo,createSeo,useWasm,wasm");
  });
});
