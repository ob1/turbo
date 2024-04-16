/// <reference path="./types.d.ts" />

const jest = __turbopack_external_require__("jest-circus");
const expectMod = __turbopack_external_require__("expect");

globalThis.describe = jest.describe;
globalThis.it = jest.it;
globalThis.test = jest.test;
globalThis.expect = expectMod.expect;

// From https://github.com/webpack/webpack/blob/9fcaa243573005d6fdece9a3f8d89a0e8b399613/test/TestCases.template.js#L422
globalThis.nsObj = function nsObj(obj) {
  Object.defineProperty(obj, Symbol.toStringTag, {
    value: "Module",
  });
  return obj;
};

const uncaughtExceptions: string[] = [];
const unhandledRejections: string[] = [];

process.on("uncaughtException", (e) => {
  uncaughtExceptions.push(String(e));
});

process.on("unhandledRejection", (e) => {
  unhandledRejections.push(String(e));
});

export default async function run() {
  await import("TESTS");

  const jestResult = await jest.run();

  // Wait a full tick for unhandledRejection handlers to run -- a microtask is not sufficient.
  await new Promise((resolve) => setTimeout(resolve, 0));

  return {
    jestResult,
    uncaughtExceptions,
    unhandledRejections,
  };
}
