# Exact-task Teacher revisits

Implementation and evidence note, 2026-11-06. The policy is `teacher-delayed-revisit-v1`, implemented in [`rufren-core/src/teacher_revisit.rs`](../crates/rufren-core/src/teacher_revisit.rs) and connected to archived teaching in [`rufren-app/src/teacher_revisit.rs`](../crates/rufren-app/src/teacher_revisit.rs). This extends [AI teacher pedagogy](AI_TEACHER_PEDAGOGY.md) and [learning models](OPTIMAL_LEARNING.md). It describes source behavior; device and browser acceptance are separate checks.

The purpose is ordinary learning over several days: return to something useful, try it before rereading the answer, receive help, and use it again later. The AI remains the primary teacher. Self-report is optional; a friend has no grading, preparation or approval duties.

## The learning workflow

A completed guided task seeds a revisit one day later even when its self-rating is `NotReported`. That means “practised; retention unknown,” not success or failure. Opening a lesson, advancing its cursor, or receiving an explanation does not seed a completed task. The earliest eligible guided attempt anchors one schedule for the exact archived document and exercise index.

The revisit preserves the original skill, question, reference meaning and acceptable-answer guidance. It opens a fresh response opportunity before the old explanation or answer is shown. An unfinished revisit resumes its own draft; a new revisit does not prefill the previous answer. Speaking requires a new take: a known previous recording cannot substitute for new production. Cached teacher audio remains useful as the original listening input.

Help remains available. After submission, the teacher can discuss the exact response and provide a model or correction. An immediate retry is useful practice, but it does not count as another successful spaced return. Missing AI feedback does not prevent saving the attempt or continuing to learn.

The four skills retain different task inputs:

| Original skill | Required input, available before responding | Additional support to record |
| --- | --- | --- |
| Reading | The Russian text needed to answer the original question | Answer-bearing translation, model response, old feedback or explanation |
| Listening | First playback of the original reference audio | Additional playback; revealed written Russian, translation or answer |
| Writing | Original meaning or situation to express | Russian response frame/model, relevant explanation or old answer |
| Speaking | Original meaning or conversational situation | Spoken or written model of the requested response, old take or explanation |

Task input is not automatically help: showing Russian in a reading question and playing the first listening stimulus are necessary. Conversely, playing a Russian model before producing that same answer supplies the target form. Availability of help and actual use of help are different facts. The old guided attempt's blanket reference-support flag is preserved on that attempt, not copied onto a fresh revisit.

Presentation evidence distinguishes purpose and session. The first listening playback is recorded as the current task's `TaskInput` and is exempt from added help. A matching current-session `Audio` presentation represents replay/model playback and qualifies as `Replay` support; other matching support presentations qualify as reference help. Thus replay/model audio alone holds the interval under this baseline, while a separately recorded text/translation reveal can require the one-day return. These categories describe the policy's assistance conditions, not an assertion that spoken models cannot reveal an answer.

## Implemented cadence

Intervals are **1, 3, 7, 14, 30, 60 and 120 days**, bounded at the last value. These are transparent engineering choices, not empirically optimal intervals or estimates of recall probability.

| Evidence on a due return | Effect |
| --- | --- |
| No recorded help, sufficient recorded delay, and explicit self-report of recall | Advance at most one rung; count a self-report-based advance |
| Same conditions with qualified, exact-task AI `Correct` | Advance at most one rung; count an AI-based advance, without manufacturing self-report or human confirmation |
| Learner `Again`, or an adverse exact-task AI opinion | Return to the one-day interval |
| `WithHelp` or recorded answer/reference help | Return to the one-day interval |
| Replay alone | Keep the current rung and interval |
| Unrated completion without usable positive evidence | Keep the current rung and interval; schedule from this completion rather than remain perpetually due |
| Recent recorded practice or matching target exposure prevents the full-delay condition | Keep the current rung and interval |
| AI `Correct` alongside another active verdict, including `Uncertain` | Keep the rung and interval, but request an earlier recheck after one day |

The rows summarize the classifier, not independent switches. Contested AI is checked first; then `Again`/adverse opinion, answer help, replay, AI correct, and self-report. A lone uncertain opinion is not an error verdict. If both eligible AI correctness and self-reported recall are present, the scheduling advance uses the AI category; both original facts remain available. The internal `ClearError` label includes learner reports and AI opinions—it is not an objective error diagnosis.

Due eligibility and the full-delay check use the review's recorded **start**, not its later submission. Opening a form early and leaving it open past the due time cannot turn that response into a delayed retrieval success. Resuming that same form retains its original start. Eligible completed returns anchor their next due date to completion time.

Early attempts never advance the ladder or postpone the existing due date. Early error/help can shorten the next interval to one day without moving an already earlier deadline later. Early practice still enters the history used to qualify subsequent delay. Responses with the same completion timestamp are grouped conservatively: mixed evidence cannot be turned into a positive result by UUID ordering, a group advances at most once, and any early start prevents advancement of that group.

Late AI feedback belongs to its original attempt. The projection can change when that evidence arrives, but the next date anchors to that attempt's completion, not network arrival. Future-dated observations are excluded until their recorded time; duplicates do not create additional learning occurrences. A later completion cannot make an early-started form eligible for advancement. Immutable self-reports, responses and source judgments are not rewritten by scheduling.

For example: practice on day 0 seeds day 1. An eligible correct return on day 1 advances to a three-day interval, due on day 4. Replay-only practice on day 4 keeps three days, due on day 7. Answer help on day 7 returns the schedule to day 8. This is an inspectable practice recipe, not evidence that forgetting follows those exact boundaries.

## What the observations establish

This is **same-task practice**. A return to the same prompt is not a new-context test. It does not establish grammar generalization, mastery of every feature in the sentence, competence in the other three skills, or the ability to sustain a whole conversation. The implementation leaves novel-context and experimental delayed-probe labels unset. Language-content feedback on a confirmed speech transcript does not assess pronunciation, stress, fluency or intelligibility.

Scheduling delay and retention evidence also differ. A task may be due one day after its earlier attempt even if the learner reread the answer recently. The policy checks available practice and matching-target exposure before the review started to qualify its delay. Exposure between start and completion is support. The first required input of the new task is excluded from prior restudy. Outside-app exposure and incomplete records remain unknown. Recorded presentation itself does not prove attention or learning.

Matching presentation facts can arrive through sync after a response is complete. If their recorded time falls within that response's start–completion window, the derived schedule must account for the support: for example, a model displayed in another session can remove eligibility for an unhelped advance. This requalification retains its presentation sources and changes the projection, **not** the saved `Attempt.cues`, self-report, response or original event bytes. Evidence discovered later is not rewritten as help the learner previously reported. A presentation timestamped after completion does not contaminate the earlier response; it may matter to a subsequent return.

AI judgments are accepted for scheduling only when bound to the actual task, context, response and relevant dimension; speaking judgments additionally require the matching confirmed transcript. This makes an opinion attributable, not infallible. Active disagreements remain visible. Generated references are excluded from generic passage review so an original listening task cannot silently become arbitrary reading, speaking or writing homework.

The live feedback check exposed a separate teaching risk: prompt v5 correctly repaired a deliberately mistaken interpretation but carried the source lesson's first-encounter note into the later return, falsely asserting first exposure and expected translation use. That output was preserved and rejected editorially. Prompt v6 explicitly treats the original note as historical and makes next actions independent of assumed UI placement. A second real response to the same synthetic mistake corrected the meaning without those false claims and rendered in the app. Its awkward next-step wording and a duplicated Russian follow-up prompt remain recorded polish issues. These two examples test a concrete context boundary, not overall model reliability or a learner outcome; [TASKS.md](../TASKS.md) records the exact receipts.

## Evidence and engineering choices

Repeated retrieval after initial learning improved one-week retention of foreign word pairs in [Karpicke and Roediger (2008)](https://doi.org/10.1126/science.1152408). The full paper and supplement are in `~/paperbin/rufren/ai-teacher-pedagogy/`. This supports revisiting successfully learned material; it does not validate this app's intervals or prove conversational transfer.

The L2 spacing synthesis by [Kim and Webb (2022)](https://doi.org/10.1111/lang.12479) supports distributed practice and reports statistically equivalent equal versus expanding schedules. Our current review of that source is abstract-based. It supplies no uniquely correct first delay or interval ladder. Seeding a next-day revisit from an unrated guided response is a practical way to offer another learning opportunity without inventing a positive assessment.

[Lyster and Saito (2010)](https://doi.org/10.1017/S0272263109990520) found benefits from classroom oral corrective feedback, including opportunities to repair responses. The local full text is in `~/paperbin/rufren/`. This motivates teaching after an attempt; it does not establish automatic Russian judgment accuracy. [Gass et al. (2019)](https://doi.org/10.64152/10125/44684), available in the pedagogy subfolder, provides evidence about caption use and individual differences, with substantial study limitations. It supports investigating flexible scaffolding, not diagnosing a memory trait or imposing one universal cue-fading rule.

The exact ladder, replay/model-audio hold, one-day error/help return, contested recheck, start-based delay eligibility, and use of AI correctness to advance are **unvalidated Rufren policy choices**. Recording the circumstances accurately makes those choices inspectable; it does not validate them. Separate counters and preserved attempts are not proficiency scores, clinical measures, or measured gains from the scheduler.

## Experiments after the useful baseline

- **Cue fading:** compare optional text, frames and replay followed by later independent use. Vary one support at a time; keep help accessible and measure effort as well as performance. A replay-permitted listening schedule may be more useful than the current conservative hold for some tasks.
- **Adaptive intervals:** compare a modest task-specific memory model against this fixed ladder using delayed outcomes and practice cost. Sparse personal history does not justify a highly parameterized model or a universal target retention percentage.
- **Changed contexts:** teach related but new messages, speakers or combinations and record their exposure separately. Distinguish learning from those new examples from assessment of transfer; assigning a new revision ID alone does not make a stimulus novel.

These are plausible next improvements, not demonstrated optimal policies. Everyday teaching and revisits should remain usable while they are investigated.
