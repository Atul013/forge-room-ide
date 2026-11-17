# Native text model connections

The AI panel now separates **Installed CLI agent** from **Native model connection**. Native
connections call a configured HTTP API directly, without Codex/Claude CLI installation. They
support text Chat Completions-compatible APIs (including Ollama/LM Studio compatibility routes)
and Anthropic Messages. This is a bounded adapter contract, not a claim that every provider,
model modality or model-specific feature has been implemented or validated live.

## Setup and request review

Choose a preset or enter an API base URL, protocol and exact model ID. Fetch models is an
explicit GET of the endpoint's model catalog; it sends no chat/file context. Model download,
server startup, account OAuth and CLI authentication are not performed by this adapter.
If authentication is needed, enter the **name** of an environment variable containing the
provider API key. Set it before starting Forge Room. Settings retain that variable name,
endpoint, model and limits, never the resolved key or native execution choice.

Use Ask, Plan or Reviewed edits and explicitly attach saved files through the existing context
workflow. Send first opens a review showing the destination URL, model, serialized request size,
output/time limits and all prompt/history text. **Send this request** transmits the reviewed
payload. Cancel sends nothing and preserves the draft. Before approval takes effect, the app
checks trust/workspace/flow and rereads attachments/plan; changed data requires a new review.
A newer composer draft typed during review is retained instead of being erased on approval.

Native requests have no native tools, commands, images, automatic workspace scanning or direct
CLI agent mode. Provider-supplied tool/function calls are rejected and never executed. Ask/Plan
responses cannot write files. Reviewed proposals use the existing exact-replacement preview,
explicit Apply, unchanged-file/dirty-buffer checks, durable recovery and guarded undo. Receipt
or generation alone does not apply a file change. Individual native tool approvals and broader
typed IDE context are still future work; this adapter does not bypass that missing capability.

## Privacy, continuation and transport

CLI messages/session IDs are never converted into native request history. Optional native
continuation retains only successful native prompt/answer turns in memory and reviews them on
every send. It is scoped to workspace, endpoint, model, protocol, key-variable identity and flow.
Switching these scopes clears it. New Chat, workspace changes and restricted mode clear it;
it is not reconstructed from saved transcripts on restart. Existing transcript retention still
stores visible user/assistant messages under the app's normal local chat policy.

HTTPS uses certificate validation. Plain HTTP is accepted only for loopback IPs/localhost;
localhost is resolved to 127.0.0.1 to avoid DNS/proxy forwarding. Redirects are disabled, system
HTTP proxies are not used, and endpoint userinfo/query/fragment credentials are rejected. The
request's credential header is marked sensitive. Errors expose HTTP status and bounded generic
failure descriptions, not raw server bodies or secret headers. A localhost endpoint may itself
use a remote/cloud model; endpoint locality does not establish offline inference.

## Limits and completion

- Request JSON: 512 KiB, including selected context and optional native history.
- Output text: 512 KiB; response body: 4 MiB; each streaming line/event: 256 KiB.
- Output-token request: 128–32768. Chat compatibility supports max_tokens or the selectable
  max_completion_tokens field; Anthropic uses max_tokens.
- Client time limit: 10–600 seconds, connection timeout five seconds. Explicit model catalog
  discovery is limited to ten seconds / 512 KiB / 100 retained IDs.
- Native history is bounded; a full conversation requires an explicit new conversation rather
  than silently dropping turns from the outgoing context.

SSE text streams are decoded across transport/UTF-8 fragment boundaries. JSON fallback responses
are accepted for compatible servers. Completion reasons and terminal stream markers must be
present. Token exhaustion, interrupted/incomplete streams, refusals, provider errors and tools
are failures: partial output is shown but never accepted as a completed edit proposal. Reasoning,
tool and non-text blocks are not displayed/executed as model text.

Stop cancels the native request; New Chat, workspace switches and removal of folder trust also
release it. Output limits are partly provider parameters, with separate enforced client byte/time
bounds. These are not financial budgets or OS isolation. A server may keep computing/billing
after its client disconnects; the app does not claim server-side cancellation or dollar caps.

## Evidence and open gates

Local HTTP stand-ins test both API schemas, fragmented streaming/UTF-8, JSON fallback, model
discovery, missing authentication, endpoint/input limits, cancellation before headers, refusal,
unsolicited tools, incomplete/truncated output and redirect refusal. App tests cover review before
send, no CLI-history forwarding, changed-file/trust refusal, exact proposal preview, explicit
apply/undo/recovery and preservation of newer composer drafts. Old settings load without the
new optional connection field. Windows CI is tracked on the PR.

No Ollama/LM Studio server was listening on the development host during this batch. Real native
inference against local/cloud providers, authenticated cloud accounts, visible request-review
controls, accessibility and desktop interaction remain open acceptance gates. The Windows
Computer Use native pipe was unavailable. Previously verified CLI/LSP behavior is not evidence
of native provider acceptance.

Protocol references: [Ollama compatibility](https://docs.ollama.com/api/openai-compatibility),
[LM Studio Chat Completions](https://lmstudio.ai/docs/developer/openai-compat/chat-completions),
[Chat Completions](https://developers.openai.com/api/reference/resources/chat),
[Anthropic streaming](https://platform.claude.com/docs/en/build-with-claude/streaming).
