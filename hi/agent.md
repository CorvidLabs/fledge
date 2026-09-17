---
hi: 1
families: [AGENT]
---

# Working with agents

## Intent

The same CLI a person uses should be usable by an AI agent, with no second interface to keep in sync. An agent should be able to learn the whole surface in one call, get structured output from anything that reports something, and never sit waiting on a prompt nobody is there to answer. Exit codes and output shapes are the contract: if an agent has to screen-scrape pretty output or guess whether something worked, that is a fault in the tool, not in the agent.

## Criteria

- **AGENT-1**  An agent can learn fledge's entire command surface in one call.
  - **AGENT-1.a**  Every flag a command accepts is listed on that command, including the ones inherited from above it.
- **AGENT-2**  Every command that reports something can report it as JSON.
  - **AGENT-2.a**  Every JSON output carries the version of the shape it is using.
  - **AGENT-2.b**  The shapes stay consistent across commands, so one parser copes with all of them.
  - **AGENT-2.c**  In JSON mode nothing but JSON reaches standard output.
- **AGENT-3**  One switch makes every prompt in fledge stop asking.
  - **AGENT-3.a**  A prompt with no sensible default fails at once rather than hanging.
  - **AGENT-3.b**  That failure names the flag I should have passed instead.
  - **AGENT-3.c**  I can turn prompting off through the environment as well as on the command line.
  - **AGENT-3.d**  Interrupting fledge at a prompt leaves my terminal the way it found it.
- **AGENT-4**  A bad command line and a command that ran and failed are told apart by exit code alone.
  - **AGENT-4.a**  A command that failed never exits zero just because its JSON printed.
  - **AGENT-4.b**  When part of a run fails but the rest succeeds, the JSON says which part failed and why.
- **AGENT-5**  An agent can read the project's module contracts as data instead of hunting through files.
- **AGENT-6**  An agent can list the commands plugins have added, since those are not part of the core surface.
- **AGENT-7**  Piping fledge's output into another tool and closing it early ends quietly, the way any Unix tool does.
