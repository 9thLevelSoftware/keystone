# Review Guide

This document is the standing guidance for reviewing pull requests in this repository. Every PR review must follow it.

## Two-Pass Review

Every pull request requires two passes, in order:

1. **Correctness review.** Bugs, edge cases, security, data-loss risk, race conditions, missing validation, bad error handling, broken tests, and regressions.
2. **Ponytail review.** A dedicated pass for unnecessary complexity. This pass is mandatory and is never optional.

If the Ponytail pass has no findings, the review must still include the section and state `Ponytail: Lean already. Ship.`

The goal is a codebase that is correct, secure, maintainable, and as small as possible. Prefer the laziest solution that actually works: fewer files, fewer dependencies, fewer abstractions, fewer branches, fewer concepts.

## General Review Order

1. **Understand intent.** Read the title, description, linked issue, and changed files. Identify the behavior that is supposed to change. Do not propose simplifications before understanding the real requirement.
2. **Correctness first.** Look for bugs, broken edge cases, security issues, data-loss risks, race conditions, missing validation, bad error handling, broken tests, and regressions. Ponytail must not strip required safety, validation, accessibility, observability, tests, or behavior explicitly requested by the author.
3. **Ponytail pass.** Search the diff for unnecessary complexity and prefer deletion over addition.

## Ponytail Pass

In the Ponytail pass, look for and flag:

- Unnecessary complexity in the diff.
- Code that should be deleted rather than added.
- Hand-rolled logic where the language standard library already provides it.
- Dependencies or custom code where the platform or native framework already provides it.
- New abstractions where an existing project pattern already covers the case.
- Factories, registries, service layers, interfaces, adapters, or config that has only one use, when one direct implementation would do.
- Speculative future-proofing.
- Code that exists "just in case."
- Abstractions with only one implementation.
- Wrappers around simple APIs.
- Dependencies used for trivial behavior.
- Helpers that duplicate what the language, framework, or repo already provides.
- Generated boilerplate or broad scaffolding the PR does not require.
- Tests that mostly exercise mocks, framework behavior, or implementation details instead of useful behavior.
- Documentation or comments that explain obvious code or defend unnecessary complexity.

### Ponytail Tags

Use these tags on findings:

- `delete` — dead code, unused flexibility, speculative feature, unnecessary branch, unused config, or scaffolding.
- `stdlib` — hand-rolled logic the language standard library already provides.
- `native` — dependency or custom code doing what the platform or framework already does.
- `yagni` — abstraction, config, or extension point with no current need.
- `shrink` — same behavior expressible with materially less code.
- `reuse` — new helper duplicates an existing project helper or pattern.
- `test-shrink` — a test can be simpler while preserving meaningful coverage.

### Ponytail Finding Format

Each finding must be concise and actionable:

```
<file>:L<line>: <tag> <what to cut>. <what replaces it>.
```

Examples:

- `src/cache.ts:L42: stdlib: custom LRU cache. Replace with Map plus size cap, or use the existing cache helper in src/lib/cache.ts.`
- `app/services/UserService.ts:L18: yagni: IUserService has one implementation and one caller. Delete the interface and inject UserService directly.`
- `src/validators/email.ts:L7: native: regex-based email parser. Use the platform/email validation already used in FormInput.`
- `tests/user.test.ts:L88: test-shrink: five mocked repository tests cover the same branch. Keep one behavior test through the public API.`
- `src/config.ts:L31: delete: FEATURE_X_STRATEGY has one value and no callers override it. Inline the value.`

If there are no Ponytail findings, the section must read exactly:

```
Ponytail: Lean already. Ship.
```

Do not invent findings. If the code is already simple, say so.

### Ponytail Boundaries

Ponytail must not propose removing:

- Required input validation.
- Security checks.
- Error handling that prevents data loss or silent failure.
- Accessibility basics.
- Tests that protect non-trivial behavior.
- Logging or metrics that are operationally necessary.
- Behavior explicitly required by the PR or linked issue.

Ponytail must not prefer a clever one-liner over a readable version when the readable version prevents mistakes. Do not block a PR only because the code could be shorter; block only for correctness, security, data-loss, or maintainability risks.

## Review Output Format

Reviews must use this structure.

### Verdict

One of:

- **Approve**
- **Request changes**
- **Comment only**

Followed by one short sentence explaining why.

### Correctness / Safety Findings

List only real correctness, safety, security, regression, or test issues.

```
<severity>: <file>:L<line>: <issue>. <required fix>.
```

Severities:

- `critical` — bug, security, or data-loss risk; must fix before merge.
- `important` — likely defect or maintainability hazard; should fix before merge.
- `minor` — small issue, typo, naming, or clarity problem.

If there are none, write exactly:

```
No correctness or safety findings.
```

### Ponytail Review

Always include this section. List findings using the exact format above, or write `Ponytail: Lean already. Ship.`

End the section with:

```
Ponytail net: -<estimated removable lines> lines.
```

If nothing is removable:

```
Ponytail net: 0 lines.
```

### Suggested Minimal Patch

If there are actionable findings, describe the smallest safe patch set. Prefer the fewest files changed, prefer deletion, introduce no new dependencies unless absolutely necessary, and avoid a broad refactor when a local fix solves the issue. Keep this section short.

If no patch is needed, write exactly:

```
No patch needed.
```

### Final Merge Guidance

State clearly whether the PR can merge, for example:

- `Can merge after the critical finding is fixed.`
- `Can merge; Ponytail suggestions are optional cleanup.`
- `Do not merge until tests cover the changed behavior.`
- `Can merge as-is.`

## Behavioral Rules

- Be direct. Be specific.
- Do not write long essays.
- Do not praise boilerplate.
- Do not ask the author to "consider" vague changes.
- Every finding must identify exactly what should change.
- Mark optional simplifications as optional.
- If a simplification is required because the complexity creates real risk, explain the risk in one sentence.
- Never treat a tool, test, or CI self-report as proof if the diff itself contradicts it.
- Prefer the smallest root-cause fix over patches scattered across callers.

## Per-PR Checklist

Before posting a review, confirm:

- Did I review correctness and security first?
- Did I run a separate Ponytail pass?
- Did I look for code to delete?
- Did I look for stdlib and native replacements?
- Did I look for one-implementation interfaces, factories, and adapters?
- Did I look for speculative config and extensibility?
- Did I avoid removing required validation, security, or tests?
- Did I include either Ponytail findings or `Ponytail: Lean already. Ship.`?