# Primary AI teaching providers

Provider documentation and pricing were checked on 2026-11-05; implementation and acceptance notes were updated on 2026-11-06. The AI teacher supplies explanations, examples, corrections, practice, review and conversation. A friend can be a casual conversation partner without preparation, recording or grading duties. Generated speech and acoustic evidence have separate contracts: see [speech providers](SPEECH_PROVIDERS.md).

## Implemented paths and runtime scope

`rufren-tutor-contract` contains network-free DTOs and validation shared by the app, core history and provider gateway. `rufren-tutor` reexports those types; its server feature owns model access. The relay owns authentication and bounded transport. Two explicit text backends are implemented:

| Backend | Configuration | Quality policy | Accounting |
|---|---|---|---|
| Existing Claude Code login | `RUFREN_TUTOR_PROVIDER=claude_cli`, `RUFREN_CLAUDE_BINARY=/absolute/path/to/claude` | `opus` alias, interactive `high` effort; actual resolved model retained | Subscription quota; no API-dollar reservation claimed |
| Anthropic HTTP API | `RUFREN_TUTOR_PROVIDER=anthropic`, server-only `ANTHROPIC_API_KEY` | Configured `claude-opus-5-5`, `max` effort and adaptive reasoning by default | Model-specific conservative API token/cost reservation |

Omitted/disabled selection leaves the service unavailable. No model credential is embedded in a browser/mobile binary. The app sends explicitly requested text and bounded context to the private relay; opening a screen does not upload a notebook. There is no automatic paid fallback, account linking, recharge or billing mutation.

The main private service now runs protocol 5, teacher-v6 and dialogue-v2 from the exact immutable executable used by the accepted v2 checks. Coordinated activation preserved private configuration, CA, TLS proxy and database identity; status confirms guided_v1/threaded_v1, local Russian speech and local Qwen ASR. The first cutover safely restarted v4 after a backup-CLI output parsing error; the repaired real-CLI boundary check and second cutover passed. No model call was made during activation. Matching native artifacts are selected; physical installation remains separate. The old executable/snapshot remain preserved, but rollback over newly written v5 history is refused. See [platform delivery](PLATFORM_DELIVERY.md) and [the tracker](../TASKS.md) for exact activation/artifact receipts.

The CLI alias follows the model available through its installed provider/account. At the original documentation check, first-party Opus resolved to 5.5; the actual recorded calls below resolved to `claude-opus-5-5`. Other explicit choices are `RUFREN_CLAUDE_MODEL=sonnet|haiku|opus` and `RUFREN_CLAUDE_EFFORT=low|medium|high|xhigh|max`. Status displays the configured family/effort, not a fictitious resolved version. `RUFREN_CLAUDE_TIMEOUT_SECONDS` defaults to 100 and accepts 1–100. Organization policy can cap effective effort. [Official model configuration](https://code.claude.com/docs/en/model-config).

## Teaching and exact-answer feedback

Authenticated routes are `GET /api/teacher/status`, `POST /api/teacher/teach` and the separate `POST /api/teacher/dialogue`. Legacy `/api/tutor/status` and `/api/tutor/suggest` remain readable/compatible general assistance paths; their mutable one-off cache is not dialogue history.

`TeacherRequest` retains its original `mode`/`input` contract:

- `teach_lesson` binds request/lesson identity, objective, material and optional learner context. Its explanation, examples, practice and next step have no answer grade. Reference-bearing practice supports reading, writing, listening and speaking. Listening uses generated audio with answer material concealed until response/reveal; speaking uses a real saved take and an independently transcribed, learner-confirmed text before language feedback. Oral references fit the speech input's 256-character / 1,024-byte bounds. Older cached practice without the additive reference fields remains readable.
- Optional `lesson_flow: "guided_v1"` opts into a bounded guided plan: 1–6 unique response-local steps, communicative goals, short orientations, optional depth, explicit example presentation roles and at most one practice per step. All practice indices appear exactly once. Listening steps keep indexed examples behind optional support. Navigation, pause and retry do not require a correct grade. Unopted requests retain the original response shape.
- `answer_feedback` binds the saved revision, attempt/context/exercise, exact prompt/reference/response, recorded cues and written/transcript evidence. Speaking also binds its exact confirmed-transcript identity. Its AI verdict is `correct`, `partly_correct`, `incorrect` or `uncertain`; it never becomes a human judgment.

The gateway fills the echoed request from validated input, outside model output. Nonempty correction originals must occur literally in the current response; empty original/replacement permit insertion/deletion, but both cannot be empty. The app checks the entire binding against immutable saved context before creating an `AiAssessmentRecorded` fact. Original accepted teaching is archived with its exact response and provenance; newer feedback does not overwrite the source lesson.

The teaching prompt is **`rufren-ai-teacher-v6`**, with `+cli-structured-v1` on the CLI path. It distinguishes the current attempt from historical guidance: a typed revisit footer and `teacher:revisit:v1:` identity describe a new response to the same saved exercise. An original first-encounter teaching note does not establish current first exposure, required assistance or memory. Empty recorded cues do not establish no outside help or independent recall. Feedback's next action is self-contained rather than dependent on hypothetical card placement.

Teaching carries `ai_generated_teaching` provenance. Legacy suggestions retain `unreviewed_ai_suggestion`. Neither is external review or measured mastery. Text, including a confirmed transcript, cannot establish pronunciation, spoken stress, fluency or intelligibility. Known sound patterns can be explained without pretending the model heard the learner.

## Learner-authored dialogue

The standalone [dialogue contract](../crates/rufren-tutor-contract/src/dialogue.rs) preserves existing lesson/feedback bytes. Status advertises optional `dialogue: "threaded_v1"`; older status payloads decode with no capability. Dialogue uses **`rufren-ai-dialogue-v2`**, independently of teacher-v6. Its dedicated system prompt does not turn conversation into a graded exercise.

Each ordinary learner message stores the complete frozen `TeacherDialogueRequest` before sending. `request_id` is its canonical logical UUID; the initial message also supplies `thread_id`. Further learner messages select an exact `parent_reply_id`. Stable scene/role configuration stays fixed while per-turn `ask`, `respond` and explicit `start_scene` actions can differ. Empty StartScene is an action, not fictional learner speech, and cannot receive a correction.

Context contains only a bounded suffix of whole messages: at most 8 ancestors / 10 KiB of serialized history. Explicit omitted-prefix identity/count records what was not sent. An optional versioned anchor selects an archived explanation, step, practice or feedback and carries its canonical bounded source projection; explicit UTF-8-safe excerpts are not generated summaries. Core checks the source, branch and exact snapshots against immutable history. Current learner text is at most 2 KiB, the anchor at most 8 KiB, and the entire serialized request at most 24 KiB. The app does not silently substitute latest history, switch anchors or upload the whole notebook.

An optional confirmed-dialogue-audio reference names the exact recording, transcription and confirmation facts. Local validation binds those facts and the confirmed words to this input; WAV bytes and worker audit data are not sent to the language model. This does not manufacture a graded `Attempt`.

The response echoes the exact request and separates English explanation, a short playable Russian utterance, a singular optional next question, optional literal current-text correction and open/closed scene state. Russian chunks fit existing speech bounds; all content fits 6 KiB. Correction preference is per-turn; no global verdict exists. The singular question field is structural, while one actual question at a time remains a prompt/editorial requirement—punctuation in quotations cannot prove communicative intent.

Replies retain their originating user/thread/parent even after navigation or a new branch. Valid competing replies remain branches rather than a clock-selected winner. Reloading or a timeout does not automatically resend a saved message. A deliberate retry can consume more quota and produce another valid reply; there is no exactly-once cost claim. Ordinary chat creates neither a review attempt nor a mastery observation.

Dialogue-v2 answers targeted beginner English help with a faithful brief meaning and usable model first; additional depth comes when requested. Grammar claims are scoped to the construction. Written word-stress marks do not determine sentence pitch; a yes/no intonation center depends on context, and no audio was heard by this text provider. These are teaching constraints, not a guarantee that future generated explanations are correct.

## Subscription-backed local Claude gateway

Rufren uses the installed, unmodified Claude Code programmatic interface. It never extracts an OAuth token for a custom HTTP client. Only an explicitly configured executable runs; native authentication is checked and bounded task JSON goes on stdin. API-key/provider-override authentication is rejected here; the HTTP adapter is the explicit API-key path. [Programmatic CLI documentation](https://code.claude.com/docs/en/headless).

An operation-private working directory, `--safe-mode`, replacement system prompt, empty ordinary tools/strict MCP configuration, disabled hooks/commands/browser integration and no session persistence isolate each operation. Only the CLI's internal `StructuredOutput` helper is allowed, with at most two schema turns. There is no private long-lived provider conversation. An environment allowlist retains necessary runtime/auth paths. Authentication and generation share a maximum 100-second deadline; stdout is capped at 128 KiB and concurrent stderr at 16 KiB, never exposed as provider prose. Unexpected tools, MCP servers, auth source, model family, malformed output or missing identity fail. Owned children are terminated/reaped; cleanup does not delete an operation directory beneath a running child.

Safe mode preserves native authentication. `--bare` and `--restricted` are not substitutes: the original investigation showed restricted mode selecting a different stale login. An isolated safe-mode public fixture succeeded on Sonnet 4.6; later actual calls verified the configured Opus/high path. Full Opus/max lessons had timed out, so interactive effort remains high with max an explicit option. Metadata-only diagnostics retain safe phase timings, counts, stream sizes and model resolution, not lesson bodies, message IDs or credentials. [CLI reference](https://code.claude.com/docs/en/cli-reference).

Legacy suggestions, lessons, feedback and dialogue share one admission gate and process request/budget state. Concurrent operations are rejected rather than queued. Subscription accounting is distinct from API prices: zero API-dollar estimates do not mean free/unlimited plan use. Reported CLI token usage aggregates up to two schema turns and is bounded at twice the per-call limit. The byte cap is not a preflight tokenizer; post-execution usage checks can still reject a result after quota was consumed.

## Anthropic API and transport bounds

The HTTP adapter uses official Messages/token-counting endpoints, server credentials and no redirects. Identical system/task/schema/reasoning configuration goes to preflight; estimated input above 7,680 tokens fails (512 headroom under 8,192). Output is capped at 8,192 tokens including adaptive reasoning and structured reply. Bounded thinking blocks are parsed, while only the structured text result is forwarded. [Messages](https://platform.claude.com/docs/en/api/messages/create), [structured outputs](https://platform.claude.com/docs/en/build-with-claude/structured-outputs), [token counting](https://platform.claude.com/docs/en/build-with-claude/token-counting), [effort](https://platform.claude.com/docs/en/build-with-claude/effort).

The following standard rates were checked on **2026-11-05**, not revalidated as an account quote. Explicit `RUFREN_TUTOR_MODEL` selections have separate configured rates:

| Model | Input / output per million tokens | Maximum reservation per attempt |
|---|---|---|
| `claude-opus-5-5` (default) | $4 / $20 | $0.196608 |
| `claude-sonnet-5-5` | $2 / $10 | $0.098304 |
| `claude-sonnet-4-6` | $3 / $15 | $0.147456 |

Unknown/unpriced model IDs fail configuration. No API caching, fast mode or tool/search/file execution is requested. `RUFREN_TUTOR_SESSION_BUDGET_MICRO_USD` defaults to 800000 ($0.80), accepts 1–5000000; `RUFREN_TUTOR_MAX_REQUESTS` defaults to 20, accepts 1–1000. An admitted attempt retains its reservation after failure because generation may have occurred. Restart resets these process-local guards; they are not persistent account limits or invoice guarantees. Prices/platform charges can change. [Original pricing source](https://platform.claude.com/docs/en/about-claude/pricing).

The original general request permits 8,192 text bytes plus 2,048 context bytes. Teacher and dialogue requests have individual field limits plus a 24-KiB aggregate bound. Serialized provider envelopes, including prompt/schema, are capped at 32 KiB; responses at 128 KiB. Combined caps may reject inputs whose individual fields fit. HTTP operations have a 100-second whole deadline and 5-second connect limit. Authenticated teacher/dialogue relay bodies are capped at 32 KiB and routes at 110 seconds, below the app's 120 seconds. Facts/assets retain 60 seconds; ASR has its separate larger upload/deadline. No automatic generation retry occurs.

## Credentials and observed evidence

A chat subscription is not a universal API credential. API-credit eligibility, linked organization, remaining balance, expiry and post-exhaustion behavior require private checking. The original investigation inspected credential presence only; no linking, billing change or API balance was claimed. The successful Claude path uses the CLI's native login. OpenAI/Gemini/Grok plans do not automatically enable an app adapter. [Original Claude API-credit help source](https://support.claude.com/en/articles/17154008-monthly-api-credits-for-max-ultra-and-team-plans).

The local ignored receipts distinguish these deliberately bounded observations:

| Check | Observed scope |
|---|---|
| Teacher-v5 guided lesson | Actual Opus/high revised lesson, 56.63s; purposeful steps and optional depth accepted after preserving an earlier problematic sample. Not teacher-v6 lesson-generation timing. |
| Teacher-v6 review feedback | Actual 15.37s written correction plus exact binding and inline UI; synthetic wrong answer and browser time, not a delayed-learning outcome. Earlier phase-confused feedback remains editorially rejected. |
| Dialogue-v1 four cases | Actual anchored explanation, empty StartScene, English help and deliberately imperfect Russian reply. Transport/UI passed; case3's unsolicited universal grammar/intonation claims were editorially rejected. |
| Dialogue-v2 two repair cases | Same original scene/ancestor context on a fresh branch; actual English help 6.494s and adapted Russian response 8.305s, both rendered. Brief meaning/model and literal хотеть→хочу correction passed scoped review. |

Source requests, raw results, prompt/executable hashes, timing, UI traces and explicit editorial limits remain under `.local/teacher-dialogue-live/` and `.local/teacher-dialogue-v2-live/`; rejected outputs were not rewritten. Those runs made exactly four plus two deliberate CLI calls, zero TTS/ASR calls and no automatic retries. Focused DTO/provider/CLI/route checks cover bounds, identity, literal corrections, shared budget, auth redaction and legacy serialized bytes. Full app/device/sync readiness belongs to their separate acceptance receipts.

The intonation caveat was checked against the Pushkin Institute proceedings' discussion of movable question centers, printed pp.109–111; this does not amount to a full pronunciation evaluation. [Primary proceedings](https://www.pushkin.institute/document/v_mire_russkogo_yazyka_thesis_2022.pdf). Passing a schema or a small editorial scene does not establish scientific efficacy, delayed transfer, native certification or general model infallibility. Paid HTTP API generation and acoustic judgments are not validated by the subscription-backed text cases.
