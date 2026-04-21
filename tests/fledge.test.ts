import { describe, it, expect } from "bun:test";

// fledge is a Rust CLI — integration tests live in tests/integration.rs.
// This file exists so the bun test harness has at least one test to run.
describe("fledge", () => {
  it("is a Rust project — see tests/integration.rs for integration tests", () => {
    expect(true).toBe(true);
  });
});
