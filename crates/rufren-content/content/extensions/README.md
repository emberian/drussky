# Conversation to ideas extension

`conversation-to-ideas-v1.json` is original AI-authored editorial content: 8 lessons,
66 phrases, 56 exercises, 16 targets and 4 grammar-source records. It extends
`ru.pack.first-contacts` revision `2026-11-05.3`; it does not replace that pack.
See [the curriculum audit](../../../../research/CURRICULUM_NEXT.md) for progression,
known gaps and assessment limits.

This is an **editorial fragment**, not a standalone Rust `ContentPack` or a runtime
plugin. Its root metadata records authorship, dependency and extension revision.
The six arrays `units`, `lessons`, `targets`, `phrases`, `exercises`, `sources` use
the existing schema-v1 item shapes. Append them in that order within their
respective base arrays to make a combined pack, preserving existing item IDs.
Keep the base pack ID, choose a new pack revision, and retain the previous revision
in `previous_revisions`. Do not copy extension-only metadata into the canonical
`ContentPack` root: that schema deliberately rejects unknown fields.

The content owner should integrate the reviewed fragment, run the canonical Rust
validator and content tests, and update counts/revision documentation. Branching
lesson prerequisites are intentional: preferences and quantities can become
available before advanced bridge material. A contract test should establish
reachable prerequisites, not require exactly one available lesson at each step.
The app may show these appended units after the original units while still allowing
an interested learner to choose available lessons.

The extension's read-only check avoids competing for a shared Rust build:

```text
python3 crates/rufren-content/content/extensions/validate.py
```

It resolves links against the base, also tolerating already-integrated items when
they are identical. A conflicting duplicate fails. It checks item shape, graph and
membership, stressed/ordinary spelling consistency, vowel placement of stress
marks, complete multisyllabic stress scaffolds, production-model hiding, actual
audio requirements, targeted review contracts, modality coverage and balanced
listening choices. It does not synthesize speech, adjudicate all grammatical
responses, or claim native-speaker certification.

Every listening/dictation activity needs supplied audio. Every speaking task is
AI-led. `coach_review` is the existing wire name for contextual AI-teacher review;
it assigns no work to a friend. Optional conversation invitations are casual meal
opportunities, with no homework, recordings or grading. All Russian phrases remain
`generated_draft`, and `pending_generated_speech` identifies the model-audio need.

The count of items is an inventory, not a proficiency claim. Seven exercise kinds
per lesson are available teaching actions, not a compulsory session checklist.
Fixed changed-message exercises are practice; only an exposure-aware host can
establish that a later assessment item was previously unseen.
