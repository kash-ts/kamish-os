## Language policy

- Internal reasoning, planning, and any chain-of-thought MUST be in English only, regardless of the user's input language.
- Tool calls, code comments, variable names, commit messages — English by default, unless explicitly asked otherwise.
- Final response to the user MUST be in Russian.
- Final response must be minimal: no filler phrases, no restating the question, no "конечно", "вот что нужно сделать", no unnecessary markdown headers/bullets unless content genuinely requires structure.
- Answer directly with the result or the question. No preamble, no summary at the end unless asked.

## Core Comment Standards

1. Language: English only (comments, TSDoc, internal log messages). Zero Cyrillic in code comments.
2. Content: Explain "why", invariants, edge cases, and concurrency guarantees. Never restate self-evident code.
3. Methods: TSDoc (`/** ... */`) in imperative/present tense for services, controllers, and domain interfaces.
4. Multiphase logic: Numbered inline comments (`// 1. ...`, `// 2. ...`).
5. Format: Space after `//`, capital first letter. Group related routes with `// --- Category ---`.

## Scope & Task Execution Policy

1. Strictly do ONLY what was requested. Never add unrequested features, extra badges, UI elements, or logic unless explicitly approved by the user.
2. Any additional or optional improvements must be proposed and agreed upon prior to implementation.
3. For every non-trivial or significant task/feature/refactor, MUST use the `/grill-me` skill (interview and clarify requirements, edge cases, and design trade-offs) before writing any code.

