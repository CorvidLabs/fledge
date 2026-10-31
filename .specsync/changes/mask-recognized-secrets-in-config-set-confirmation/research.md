---
change: mask-recognized-secrets-in-config-set-confirmation
artifact: research
---

# Research

Read the actual Set/Get/List handlers, existing classifier/set/load/save, accepted config constraints and SETUP-2.g, complete four-file diff and new real-CLI test. No external research or model-provider call was needed.

The classifier recognizes github.token and ai.anthropic/openai/claude/ollama.api_key. Reuse avoids classifier drift. The actual serializer and atomic owner-only save implementation remain unchanged. Unknown-key errors name the key and hint without interpolating its value.

Receipt20261016T070322Z-F1-cfcda0 executes the intended seven cases before the product edit: five fail on actual raw-marker stdout; two controls pass. Receipt20261016T070640Z-F1-9eb793 passes the same seven cases after the fix. Passing cases read owned TOML to prove exact marker persistence and masked Get/List. Baseline failing cases stop at the leak assertion, so their later persistence assertions are not claimed as passed.

Native lane envelopes expose successful steps but no complete test totals. Cached no-fetch audit remains failed on unchanged main dependencies:11vulnerabilities and two unmaintained warnings. PR538's49.0.2 candidate is not qualified here. Actual Trust/risk/provenance, live ownership and active lifecycle coverage remain coordinator gates.
