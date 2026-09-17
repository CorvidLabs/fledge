---
hi: 1
families: [AI]
---

# Choosing a model and asking it things

## Intent

Which model answers you should be your choice and should take one line to change — a model on your own machine, a paid API, or any gateway in between — with no vendor CLI to install first. fledge should work the moment you try it with nothing configured, and should always be able to tell you what is active and where that setting came from. Asking a question about the codebase should already carry the project's own contracts, so the answer is about this code rather than code in general.

## Criteria

- **AI-1**  I can switch which model answers me in one line.
  - **AI-1.a**  I can pick the provider and model from a list when I do not remember the names.
  - **AI-1.b**  I can set both without any prompting when I am scripting it.
  - **AI-1.c**  I can override the provider or the model for a single command without changing what is stored.
- **AI-2**  With nothing configured at all, fledge still finds a model to talk to.
  - **AI-2.a**  A model running locally with no API key is enough to get started.
- **AI-3**  I can see which provider, model and host are active right now.
  - **AI-3.a**  I can see whether each of those values came from a flag, my environment, my config or a default.
- **AI-4**  I can list the models a provider will actually accept.
  - **AI-4.a**  When that list cannot be fetched, I am told which endpoint was tried.
  - **AI-4.b**  When that list cannot be fetched, I am told the likeliest reason, such as the local model daemon not running.
- **AI-5**  Using a model never requires installing that vendor's own command-line tool.
  - **AI-5.a**  I can point fledge at any endpoint that speaks a provider's protocol, including one I host myself.
- **AI-6**  I can ask a question about this codebase in plain English and get an answer grounded in it.
  - **AI-6.a**  The model is handed a short index of the project's module contracts so it can cite them.
  - **AI-6.b**  I can pull the full contract for named modules into the question when the reasoning behind the code matters.
  - **AI-6.c**  Asking about a module that does not exist fails clearly instead of quietly answering without it.
