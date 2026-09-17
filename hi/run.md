---
hi: 1
families: [RUN]
---

# Running the project

## Intent

The words for building and testing should be the same in every repository, whatever the language, and fledge should already know them without being configured. When the guesses are not right, a small file in the project takes over completely and becomes the shared truth for everyone who works there. A task runner that runs the wrong thing, or lets a failure look like a success, is worse than no task runner at all.

## Criteria

- **RUN-1**  I can run this project's tests without knowing what language it is written in.
  - **RUN-1.a**  fledge works out the project's stack from the files that are already there.
  - **RUN-1.b**  A JavaScript project is run with the package manager its lockfile implies.
  - **RUN-1.c**  I can override the detected stack when the guess is wrong.
- **RUN-2**  I can see which tasks this project offers before I run one.
- **RUN-3**  I can write the project's tasks down once so that everyone runs the same commands.
  - **RUN-3.a**  fledge can write that file for me, pre-filled with what it detected.
  - **RUN-3.b**  Once that file exists it is the whole truth, with nothing quietly mixed in from detection.
  - **RUN-3.c**  A task can declare the tasks that must run before it.
  - **RUN-3.d**  Tasks that depend on one another in a circle are reported as an error instead of looping forever.
- **RUN-4**  I can pass extra arguments through to a task's command for a single run.
  - **RUN-4.a**  Arguments I pass through reach the command as separate arguments rather than being spliced into the command text.
- **RUN-5**  A task that fails makes the command fail.
- **RUN-6**  I can have a task re-run itself every time I save a file.
  - **RUN-6.a**  A burst of saves causes one re-run rather than ten.
  - **RUN-6.b**  Build output and version-control directories never set off a re-run.
  - **RUN-6.c**  I can watch a whole pipeline the same way I watch a single task.
