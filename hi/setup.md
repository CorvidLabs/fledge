---
hi: 1
families: [SETUP]
---

# Getting set up

## Intent

fledge should be one command away on whatever machine you are working on today — your laptop, a colleague's, a CI runner — and once it is there it should already know who you are, so scaffolding and shipping never ask you the same three questions again. When something in the environment is missing or misconfigured, fledge should name the thing and hand you the fix rather than failing halfway through a real task. Getting set up should be boring, and staying set up should take no thought at all.

## Criteria

- **SETUP-1**  I can install fledge with a single command on whichever platform I work on.
  - **SETUP-1.a**  I can add fledge to a CI workflow in one step.
  - **SETUP-1.b**  I can pin that CI step to an exact version so a later release cannot change my build.
  - **SETUP-1.c**  A binary downloaded during that step is checked against the release's published checksum before anything runs it.
  - **SETUP-1.d**  An install that cannot verify a checksum stops rather than falling back to an unverified binary.
- **SETUP-2**  fledge remembers my name, my GitHub org and my preferred license so I stop retyping them into every new project.
  - **SETUP-2.a**  I can set all of that up at once from a named preset instead of key by key.
  - **SETUP-2.b**  I can read any setting from the command line without opening the file.
  - **SETUP-2.c**  I can change any setting from the command line.
  - **SETUP-2.d**  I can clear a setting I no longer want.
  - **SETUP-2.e**  I can change one entry of a setting that holds a list without rewriting the whole list.
  - **SETUP-2.f**  A setting key I mistype is refused instead of being silently stored somewhere nothing reads.
  - **SETUP-2.g**  My stored API keys are masked when I print my settings.
  - **SETUP-2.h**  I can find out where my settings are kept when I want to look at the file myself.
- **SETUP-3**  One command tells me whether this machine is ready to work on this project.
  - **SETUP-3.a**  Every failing check comes with the command that would fix it.
  - **SETUP-3.b**  A language toolchain this project does not use is reported as absent without being counted as a failure.
  - **SETUP-3.c**  A missing API key, an unreachable model host and a wrongly configured provider read as three different problems.
- **SETUP-4**  fledge picks up my GitHub credentials from wherever I already keep them.
- **SETUP-5**  A value I pass on the command line wins over my config file, which wins over fledge's built-in defaults.
- **SETUP-6**  I can get tab completion for fledge in the shell I actually use.
