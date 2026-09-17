---
hi: 1
families: [LANE]
---

# Pipelines

## Intent

The checks that gate a merge should be written down once in the project and runnable anywhere — the same pipeline on my laptop as in CI, called by one short name. Steps should compose out of what is already there: a task, a one-off command, a group that runs at the same time, with the ordinary realities of real pipelines available without a plugin or a second config language. When a pipeline fails you want to know which step, how long it took, and be able to pick up from where it stopped instead of starting over.

## Criteria

- **LANE-1**  I can run this project's whole check pipeline with one short command.
  - **LANE-1.a**  The same pipeline behaves the same way on my laptop as it does on a CI runner.
- **LANE-2**  I can chain the project's tasks into a named pipeline without learning another language.
  - **LANE-2.a**  A step can run one of the project's tasks.
  - **LANE-2.b**  A step can run a one-off command that is not worth naming as a task.
  - **LANE-2.c**  A group of steps can run at the same time.
  - **LANE-2.d**  A step naming a task that does not exist is caught before anything runs.
  - **LANE-2.e**  I can check every pipeline definition in the project for mistakes without running any of them.
  - **LANE-2.f**  fledge can write a sensible starting pipeline for this kind of project.
- **LANE-3**  A pipeline stops at the first failure by default.
  - **LANE-3.a**  I can ask instead for a pipeline that runs everything and reports every failure at the end.
- **LANE-4**  I can see a pipeline's plan without running it.
- **LANE-5**  I can restart a pipeline from a step I choose instead of redoing what already passed.
- **LANE-6**  A step can be skipped unless a condition about the environment holds.
  - **LANE-6.a**  A skipped step tells me why it was skipped.
- **LANE-7**  A step that hangs is killed after a deadline I set.
  - **LANE-7.a**  Nothing a killed step started is left running behind it.
- **LANE-8**  A step that fails for flaky reasons can retry on its own.
- **LANE-9**  I can see how long each step took.
  - **LANE-9.a**  I can see how long the whole pipeline took.
- **LANE-10**  I can import a set of pipelines somebody else published rather than writing my own from scratch.
  - **LANE-10.a**  Importing never overwrites a pipeline I already have under that name.
  - **LANE-10.b**  I can search for published pipelines that suit this kind of project.
  - **LANE-10.c**  I can publish my own pipelines for other people to import.
