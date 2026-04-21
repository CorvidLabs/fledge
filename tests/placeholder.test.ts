// Placeholder test file — fledge is a Rust project; Rust tests run via cargo test.
// This file exists so that bun test does not exit with an error when run in this repo.
import { describe, it, expect } from "bun:test";

describe("fledge", () => {
  it("is a Rust project", () => {
    expect(true).toBe(true);
  });
});
