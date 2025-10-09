While Codex defaults to running commands automatically inside of a sandbox (limiting file writes to the workspace and disabling network access), users and admins want to configure Codex to be able to run some commands outside of that sandbox.

For example, part of Codex’s standard workflow to solve a problem is running tests, but often those tests require network access in some form. This proposal introduces `command_rules` that exposes this configurability to developers, using [`execpolicy`](https://github.com/openai/codex/tree/main/codex-rs/execpolicy) under the hood.

## `command_rules`

While `execpolicy` uses [Starlark](https://bazel.build/rules/language) (python-like configuration language syntax), there would be `[[command_rules]]` in a user’s `~/.codex/config.toml` file. Each rule describes the `argv` pattern it cares about and any overrides for approval, sandboxing, network, or hooks.

```toml
[[command_rules]]
pattern = ["kubectl", "get"] # argv prefix match
result = "allow" # "allow" | "ask-user" | "deny"
reason = "Allow Codex to get Kubernetes services"
sandbox = "danger-full-access" # "workspace-write" | "danger-full-access"
network = "enabled" # "disabled" | "enabled"
```

Every match also carries the argument metadata that `execpolicy` emits—flags, options (flags that consume a value), and positional args labeled as `ReadableFile`, `WriteableFile`, literal subcommands, and so on. That lets rules and hooks stay precise:
- Downgrade to `ask-user` whenever a `WriteableFile` target resolves outside the workspace.
- Deny `kubectl delete` or `rm -rf` based on literal arguments even if the `program` name matches a broader allow rule.
- Inspect `MatchedOpt` entries to catch risky options such as `--output=/etc/passwd` before auto-approving.
- Build a hook that inspects the JSON payload (received on stdin) and refuses auto-approval when it spots dangerous combinations like `--allow-root` or `--dangerous`.

```toml
[[command_rules]]
pattern = ["pip", "install"]
result = "ask-user"
reason = "Prompt if pip tries to edit system packages"
network = "enabled"

[[command_rules]]
pattern = ["pip", "install", "--target"]
result = "deny"
reason = "Refuse when --target points outside the workspace"
```

The first rule allows routine installs but keeps them interactive. The second rule piggybacks on the `MatchedOpt("--target")` classification: when the option is present and its value resolves outside the project, `execpolicy` marks it as a `WriteableFile`, causing the deny rule to win.

When multiple rules match the same command, Codex always takes the safest option for each field: `deny` > `ask-user` > `allow`, `read-only` > `workspace-write` > `danger-full-access`, `disabled` > `enabled`. If no rule touches a field, Codex falls back to the standard behavior listed above. Reasons are merged the same way—the most restrictive explanation wins so the UI explains why the command ran with its final guardrails.

When a command matches a rule, we surface the `reason` (if present) inside the prompt/auto approval toast so users know why elevated access was granted.

Rules can cover broad families by matching only the program name (`["cargo"]`) or include placeholders in the Starlark policy itself; this TOML layer only handles argv prefix matching to keep it simple and auditable.

### **Managed Overrides**

When Codex runs in a managed environment, admins can ship a higher-precedence config (e.g. `/etc/codex/config.toml`). User-level settings are merged on top only when they do not conflict. For example:

```toml
# Admin policy (enforced first)[[command_rules]]
pattern = ["b", "list"]
result = "allow"
sandbox = "workspace-write"
network = "enabled"
reason = "Corp policy: diagnostics stay in workspace-write"

# User config (applied second)[[command_rules]]
pattern = ["b", "list"]
result = "allow"
sandbox = "danger-full-access"
reason = "I trust `b list` to need network"
```

Both configs match `["b","list"]`. The resolver keeps `result = "allow"` (no conflict), but it tightens the sandbox to `workspace-write` and network to `enabled`, reflecting the stricter admin stance. The prompt/logs cite the admin reason so users understand the enforced limit.

### **Executable Hooks**

Rules can delegate the final auto-approval decision to an executable. Set `hook` to a script path; Codex streams invocation context (argv, cwd, environment hints) to its stdin. The hook must:

- Exit with code `0`
- Emit JSON on the final line of stdout: `{"reason":"…","result":"allow"}`

Only when both conditions hold does Codex keep the auto-approval, reusing the JSON `reason` in the UI. Any other exit code or JSON payload downgrades the decision to a prompt, and we show the hook’s stdout/stderr to explain why.

Example: before auto-running `pytest`, run a shell script that diffs the repo and asks an internal `openai` helper whether the changes look risky.

```toml
[[command_rules]]
pattern = ["pytest"]
result = "allow"
network = "enabled"
sandbox = "workspace-write"
hook = "/usr/bin/security-managed/review-pytest.sh"
reason = "Run pytest automatically when changes look safe"
```

`review-pytest.sh` might collect `git diff`, call an audited CLI that submits the diff to OpenAI, and print a final JSON line such as `{"reason":"No changes that affect network access", "result":"allow"}` before exiting with code `0`. Any other outcome leaves pytest gated behind a manual approval.
