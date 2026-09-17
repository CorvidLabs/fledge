---
hi: 1
families: [SCAFFOLD]
---

# Starting a project

## Intent

Starting a new project should not mean copying the last one and fixing whatever broke on the way. One command should produce something already wired for tasks, pipelines and releases, whether the starting point ships with fledge or lives in somebody's repository on GitHub. Because a starter can run commands on your machine, you should always see them before they run, and trust should be something you grant once for a source you know rather than something you are nagged about forever. Anyone who has a project layout worth reusing should be able to turn it into a starter other people can find.

## Criteria

- **SCAFFOLD-1**  I can go from nothing to a working project in one command.
  - **SCAFFOLD-1.a**  The project I get is already wired for its tasks, its pipelines and its releases.
  - **SCAFFOLD-1.b**  I can pick a starter from a list when I do not know what is on offer.
  - **SCAFFOLD-1.c**  I can start from any GitHub repository, not only the starters that ship with fledge.
  - **SCAFFOLD-1.d**  I can pin a remote starter to a tag or branch so the same command gives me the same project later.
  - **SCAFFOLD-1.e**  I can see everything a scaffold would create before it writes a single file.
- **SCAFFOLD-2**  The scaffold asks me only for the things it cannot work out on its own.
  - **SCAFFOLD-2.a**  Answers already sitting in my settings are filled in for me rather than asked again.
- **SCAFFOLD-3**  Setup commands a starter wants to run on my machine are shown to me before they run.
  - **SCAFFOLD-3.a**  A starter I passed in off the internet never runs commands without a separate, explicit yes from me.
  - **SCAFFOLD-3.b**  A source I have already chosen to trust does not ask me again every time.
  - **SCAFFOLD-3.c**  Skipping those commands still leaves me a scaffolded project.
  - **SCAFFOLD-3.d**  A scaffold that skipped setup commands tells me which ones it skipped.
- **SCAFFOLD-4**  I can turn a project layout I like into a starter other people can use.
  - **SCAFFOLD-4.a**  I can check a starter for mistakes before anyone else runs it.
  - **SCAFFOLD-4.b**  I can publish my starter somewhere other people will find it.
- **SCAFFOLD-5**  I can search for starters other people have published.
