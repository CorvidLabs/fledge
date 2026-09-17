---
hi: 1
families: [PLUGIN]
---

# Extending fledge

## Intent

The core stays small on purpose: anything that assumes one ecosystem — GitHub, lockfile audits, code metrics — lives outside it and installs in one command. Installing somebody else's code is a decision, so before it lands you should see who published it, what it wants access to, and what it will run. A plugin that only needs to think about your files should be able to run sandboxed, with reaching your disk or the network granted rather than assumed. Writing and sharing one should be as easy as installing one.

## Criteria

- **PLUGIN-1**  I can install the curated set of extras in one command.
  - **PLUGIN-1.a**  One bad plugin in that set does not stop the others from installing.
- **PLUGIN-2**  I can install a plugin from GitHub, from a git URL, or from a directory on my own disk.
  - **PLUGIN-2.a**  I can pin a plugin to a version.
  - **PLUGIN-2.b**  An update leaves a pinned plugin exactly where I pinned it.
  - **PLUGIN-2.c**  I can remove a plugin and have its commands go with it.
- **PLUGIN-3**  Before a plugin is installed I am shown who published it, what it wants access to, and what it will run.
  - **PLUGIN-3.a**  I can see where a source sits on the trust scale — my own, official, a known colleague, or a stranger.
  - **PLUGIN-3.b**  I can extend that trust scale to my own org and my own colleagues.
  - **PLUGIN-3.c**  A stranger's plugin asking to run commands or reach the network is refused outright.
  - **PLUGIN-3.d**  A source dressed up to look as though it comes from a trusted org is treated as a stranger's.
  - **PLUGIN-3.e**  A plugin cannot quietly gain new access when it updates.
- **PLUGIN-4**  I can check at any time what every installed plugin is allowed to do.
- **PLUGIN-5**  A plugin can run fully sandboxed, reaching neither my files nor the network unless I grant it.
  - **PLUGIN-5.a**  A sandboxed plugin cannot run forever.
  - **PLUGIN-5.b**  A sandboxed plugin cannot eat unbounded memory.
  - **PLUGIN-5.c**  A sandboxed plugin behaves the same on every machine I work on.
- **PLUGIN-6**  I can scaffold a plugin of my own without studying the protocol first.
  - **PLUGIN-6.a**  I can validate it before anybody else installs it.
  - **PLUGIN-6.b**  I can publish it for other people to install.
- **PLUGIN-7**  A plugin can ask me questions without shipping its own interface.
  - **PLUGIN-7.a**  A plugin can report progress through fledge rather than printing over it.
  - **PLUGIN-7.b**  A plugin can keep state between runs without inventing its own storage.
- **PLUGIN-8**  A plugin's command emits the same JSON envelope as a core command.
- **PLUGIN-9**  I can find out which plugins suit this project rather than hearing about them by word of mouth.
