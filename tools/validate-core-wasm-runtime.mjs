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
const fixed = instantiate("fixed-width-core");
const answer = numeric.exports.main();
const whenTrue = control.exports.select(1);
const whenFalse = control.exports.select(0);

if (answer !== 42n || whenTrue !== 7n || whenFalse !== 9n) {
  throw new Error(
    `unexpected results: numeric=${answer}, true=${whenTrue}, false=${whenFalse}`,
  );
}

const I64_MIN = -(1n << 63n);
const I64_MAX = (1n << 63n) - 1n;
const U64_MAX = (1n << 64n) - 1n;

function expectedSigned(left, right, add) {
  const mathematical = add ? left + right : left - right;
  if (mathematical > I64_MAX) return [1, 0n];
  if (mathematical < I64_MIN) return [1, 1n];
  return [0, mathematical];
}

function expectedUnsigned(left, right, add) {
  const mathematical = add ? left + right : left - right;
  if (mathematical > U64_MAX) return [1, 0n];
  if (mathematical < 0n) return [1, 1n];
  return [0, mathematical];
}

function assertChecked(actual, expected, unsigned, label) {
  const [actualTag, rawPayload] = actual;
  const actualPayload = unsigned
    ? BigInt.asUintN(64, rawPayload)
    : BigInt.asIntN(64, rawPayload);
  if (actualTag !== expected[0] || actualPayload !== expected[1]) {
    throw new Error(
      `${label}: expected=${expected[0]},${expected[1]} actual=${actualTag},${actualPayload}`,
    );
  }
}

const boundaries = [
  ["i64 add max", fixed.exports.i64_checked_add(I64_MAX, 1n), [1, 0n], false],
  ["i64 add min", fixed.exports.i64_checked_add(I64_MIN, -1n), [1, 1n], false],
  ["i64 sub max", fixed.exports.i64_checked_sub(I64_MAX, -1n), [1, 0n], false],
  ["i64 sub min", fixed.exports.i64_checked_sub(I64_MIN, 1n), [1, 1n], false],
  ["u64 add max", fixed.exports.u64_checked_add(U64_MAX, 1n), [1, 0n], true],
  ["u64 sub zero", fixed.exports.u64_checked_sub(0n, 1n), [1, 1n], true],
];
for (const [label, actual, expected, unsigned] of boundaries) {
  assertChecked(actual, expected, unsigned, label);
}

let state = 0x9e3779b97f4a7c15n;
for (let index = 0; index < 2_048; index += 1) {
  state = BigInt.asUintN(64, state * 6364136223846793005n + 1442695040888963407n);
  const unsignedLeft = state;
  state = BigInt.asUintN(64, state * 6364136223846793005n + 1442695040888963407n);
  const unsignedRight = state;
  const signedLeft = BigInt.asIntN(64, unsignedLeft);
  const signedRight = BigInt.asIntN(64, unsignedRight);

  assertChecked(
    fixed.exports.i64_checked_add(signedLeft, signedRight),
    expectedSigned(signedLeft, signedRight, true),
    false,
    `property ${index} signed add`,
  );
  assertChecked(
    fixed.exports.i64_checked_sub(signedLeft, signedRight),
    expectedSigned(signedLeft, signedRight, false),
    false,
    `property ${index} signed sub`,
  );
  assertChecked(
    fixed.exports.u64_checked_add(unsignedLeft, unsignedRight),
    expectedUnsigned(unsignedLeft, unsignedRight, true),
    true,
    `property ${index} unsigned add`,
  );
  assertChecked(
    fixed.exports.u64_checked_sub(unsignedLeft, unsignedRight),
    expectedUnsigned(unsignedLeft, unsignedRight, false),
    true,
    `property ${index} unsigned sub`,
  );
  if (
    fixed.exports.i64_equal(signedLeft, signedRight) !== Number(signedLeft === signedRight) ||
    fixed.exports.i64_less_than(signedLeft, signedRight) !== Number(signedLeft < signedRight) ||
    fixed.exports.u64_equal(unsignedLeft, unsignedRight) !== Number(unsignedLeft === unsignedRight) ||
    fixed.exports.u64_less_than(unsignedLeft, unsignedRight) !== Number(unsignedLeft < unsignedRight)
  ) {
    throw new Error(`property ${index}: fixed-width comparison mismatch`);
  }
}

const componentOutIndex = process.argv.indexOf("--component-out");
if (componentOutIndex >= 0) {
  const output = process.argv[componentOutIndex + 1];
  const component = snapshots.get("fixed-width-component");
  if (output === undefined || component === undefined) {
    throw new Error("--component-out requires a path and fixed-width-component snapshot");
  }
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, Buffer.from(component, "hex"));
}

process.stdout.write(
  `CORE_WASM_RUNTIME_OK engine=node-${process.version} numeric=${answer} control_true=${whenTrue} control_false=${whenFalse} fixed_properties=2048x8 boundaries=${boundaries.length}\n`,
);
