---
hi: 1
families: [SPEC]
---

# Module contracts

## Intent

Design documents rot because nothing ever checks them. Every module should be able to declare in writing what it is for and what it guarantees, and that declaration should be checkable against the code so drift fails the build instead of quietly accumulating for a year. The same contracts are what the AI features read, so writing one pays off twice: once for the next person, once for the model reviewing your diff.

## Criteria

- **SPEC-1**  Each module in my project can carry a written contract saying what it is for and what it guarantees.
- **SPEC-2**  I can check every contract against the code in one command.
  - **SPEC-2.a**  A contract pointing at a file that no longer exists is an error.
  - **SPEC-2.b**  A module with no contract at all is reported.
  - **SPEC-2.c**  A failing check fails the build, so drift cannot land.
  - **SPEC-2.d**  I can ask for a stricter check that treats warnings as errors.
- **SPEC-3**  I can scaffold a new module's contract instead of remembering its shape.
  - **SPEC-3.a**  Scaffolding never overwrites a contract that is already there.
- **SPEC-4**  I can list every contract in the project.
  - **SPEC-4.a**  I can read any single contract in detail.
- **SPEC-5**  One malformed contract does not break the commands that merely read contracts.
- **SPEC-6**  Checking contracts locally gives me the same verdict CI will give me.
- **SPEC-7**  I can start using contracts in a project that has none without hand-writing the setup.
