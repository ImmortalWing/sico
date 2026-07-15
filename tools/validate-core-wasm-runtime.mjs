import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repository = path.resolve(import.meta.dirname, "..");
const snapshotPath = path.join(repository, "tests", "wasm", "artifacts.hex");
const snapshots = new Map(
  fs
    .readFileSync(snapshotPath, "utf8")
    .trim()
    .split(/\r?\n/u)
    .filter(Boolean)
    .map((line) => {
      const separator = line.indexOf("=");
      if (separator <= 0) {
        throw new Error(`invalid snapshot line: ${line}`);
      }
      return [line.slice(0, separator), line.slice(separator + 1)];
    }),
);

function instantiate(name) {
  const hex = snapshots.get(name);
  if (hex === undefined) {
    throw new Error(`missing ${name} Core Wasm snapshot`);
  }
  const bytes = Buffer.from(hex, "hex");
  if (!WebAssembly.validate(bytes)) {
    throw new Error(`${name} failed the engine validator`);
  }
  return new WebAssembly.Instance(new WebAssembly.Module(bytes));
}

const numeric = instantiate("numeric");
const control = instantiate("control");
const answer = numeric.exports.main();
const whenTrue = control.exports.select(1);
const whenFalse = control.exports.select(0);

if (answer !== 42n || whenTrue !== 7n || whenFalse !== 9n) {
  throw new Error(
    `unexpected results: numeric=${answer}, true=${whenTrue}, false=${whenFalse}`,
  );
}

process.stdout.write(
  `CORE_WASM_RUNTIME_OK engine=node-${process.version} numeric=${answer} control_true=${whenTrue} control_false=${whenFalse}\n`,
);
