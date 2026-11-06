# Evidence for Rufren

Initial targeted review, 2026-11-05. Evidence for a component does not validate the combined app. Cross-language findings motivate Russian implementation tests; study effect sizes are not promised gains for Ember. Full-text copies are in `~/paperbin/rufren/` where acquisition succeeded. Abstract-only findings are identified below.

The follow-up [efficient learning review](OPTIMAL_LEARNING.md) covers optimal scheduling, POMDP teaching, Bayesian pedagogy, interleaving, and experimental curriculum selection. The [Rust platform assessment](RUST_PLATFORM.md) supersedes the initial Apple-only implementation recommendation.

## Memory and transfer

**E1 — Kim and Webb (2022), spaced practice meta-analysis.** 48 experiments, 3,411 participants, 98 effect sizes. The publisher abstract reports medium-to-large spacing benefits; longer intervals helped delayed outcomes more than shorter ones, and equal versus expanding schedules were statistically equivalent. Use spaced retrieval, but do not claim a uniquely optimal algorithm. Access: publisher abstract and metadata. [Paper](https://doi.org/10.1111/lang.12479).

**E2 — Karpicke and Roediger (2008), foreign vocabulary retrieval experiment.** Repeated retrieval after initial successful learning improved delayed recall in the studied task; additional study after learning did not provide the same benefit. This supports revisiting successfully retrieved vocabulary, not endless testing of material the learner cannot yet retrieve. Access: indexed abstract. [Paper](https://doi.org/10.1126/science.1152408).

**E7 — Yanagisawa (2016), receptive and productive vocabulary retrieval.** Productive retrieval gave the strongest productive test result; receptive and productive retrieval performed similarly on the receptive test. Use modality-specific evidence. This is a bounded vocabulary experiment, not proof of broad conversational transfer. Access: publisher abstract. [Paper](https://doi.org/10.20806/katejournal.30.0_139).

## Speech perception, production, and feedback

**E3 — Uchihara, Karas, and Thomson (2025), HVPT perception meta-analysis.** 79 studies; treatment-control effect g = 0.67, versus pre/post g = 0.92. The protocol combines varied talkers, phonetic contexts, and feedback. Supports perceptual training and delayed/generalization assessment. The combined effect cannot be attributed solely to more speakers, and moderator findings do not prescribe an optimal speaker count for Ember. Access: full text, abstract, methods and discussion sections. [Paper](https://doi.org/10.1017/S0272263125100879).

**E4 — Uchihara, Karas, and Thomson (2024), perception-to-production meta-analysis.** 31 studies found small-to-medium production benefits from perceptual HVPT, with limited support for durable production gains and generalization to untrained stimuli. Participant-level perception/production improvement was weakly related. Practice and assess speaking as well as hearing. Access: full text, abstract and relevant discussion. [Paper](https://doi.org/10.1017/S0142716424000195).

**E5 — Lyster and Saito (2010), classroom oral corrective feedback meta-analysis.** 15 studies, 827 participants. Feedback benefited target-language development, with prompts showing larger effects than recasts in the synthesis. This motivates opportunities to notice and repair a response. Classroom findings do not establish the reliability of AI feedback or justify interrupting every conversational error. Access: full-text copy and publisher abstract. [Paper](https://doi.org/10.1017/S0272263109990520).

**E6 — Roon, Kang, and Whalen (2020), Russian palatalization familiarization.** 18 English-speaking learners with no prior Slavic-language study. Ultrasound and audio-only groups both improved on word-final production and discrimination, without significant between-group differences in those improvements. A more specific manner-discrimination result favored ultrasound. This is not evidence that advanced displays generally outperform good auditory coaching. Correction after reading Hacking et al.'s (2017) primary full text: despite Roon et al.'s contrary summary, Hacking §5.3 reports native-listener identification increasing from 60% to 68%, statistically significantly across 10 learners, t(9) = 3.480, p < .01. This modest result from EPG training does not establish general phonemic mastery or validate AI pronunciation assessment. Access: Roon full text, abstract, participants and production results; Hacking primary full text, §5.3 and Table 5. [Roon et al.](https://doi.org/10.1159/000505298), [Hacking et al. primary PDF](https://eric-m-johnson.com/wp-content/uploads/2021/05/hacking-smith-johnson-2017.pdf).

## Russian script and grammar

**E8 — Hayes-Harb and Barrios (2021), orthography and phonological acquisition review.** Written input can influence sound learning; effects depend on the writing system, prior letter-sound associations, and perceptibility. Russian studies discussed in the review show that spelling/metalinguistic knowledge can outpace auditory knowledge and that stress marks alone need not solve stress learning. Teach Cyrillic with sound and separately test listening; the evidence does not establish a universal prohibition on text or transliteration. Access: full text, Russian palatalization and stress sections. [Paper](https://doi.org/10.1017/S0261444820000658).

**E9 — Comer and deBenedette (2011), processing instruction for Russian.** The publisher abstract reports that processing instruction improved interpretation and production of locational/directional expressions, while traditional mechanical practice benefited production most. Use comprehension where form changes meaning alongside production. Generalization to every Russian structure remains untested here. Access: publisher abstract. [Paper](https://doi.org/10.1111/j.1944-9720.2011.01155.x).

**E10 — Kisselev and Rubina (2024), Russian case pedagogy.** Reviews learning challenges and textbook approaches and proposes usage-based instruction with sample accusative materials. Useful for curriculum design; it is not a randomized efficacy trial of that proposed curriculum. Access: publisher abstract and full-text copy, opening and pedagogical framing. [Paper](https://doi.org/10.70163/0036-0252.1397).

## What abcdeez contributes

**A1 — Local design archaeology.** `~/dev/abcdeez/PAPER.md` proposes future graph-learning experiments, retention and transfer tests. It does not report validation. The current `core/src/learner.rs` distinguishes operations and confusions; its information-gain function is response entropy times hand-coded difficulty. Diagnostic uncertainty reduction and improvement of human learning are different objectives.

Carry forward separate skills, concrete confusion tracking, delayed assessment, and transfer tests. Do not inherit graph traversal as a complete model of language, a fixed success-rate target, or README completion claims as evidence. The abcdeez working tree has substantial unrelated edits and was left untouched. Conversation archaeology found implementation/audit discussions rather than an original substantive learning-science consultation; the absence of a recovered consultation does not prove none existed.

## Implementation sources

**T1 — Apple multiplatform targets.** Shared targets and SwiftUI support the proposed Apple-only architecture. [Documentation](https://developer.apple.com/documentation/xcode/configuring-a-multiplatform-app-target).

**T2 — Apple live spectrogram example.** AVFoundation, vDSP, and vImage demonstrate local audio capture and visualization. The sample is technical feasibility evidence, not a pronunciation assessment model. [Documentation](https://developer.apple.com/documentation/accelerate/visualizing-sound-as-an-audio-spectrogram).

**T3 — AI access, checked 2026-11-05.** Anthropic documents monthly API credits for eligible Max, Ultra, and Team plans after linking a Console organization. OpenAI also documents an API-credit benefit for eligible ChatGPT Pro and Business subscriptions. Neither establishes which benefit is active in Ember's accounts. Gemini's API has its own project/billing configuration. Grok API entitlements have not been established. Use actual account balances and model capabilities before estimating operating cost. [Claude credits](https://support.claude.com/en/articles/17154008-monthly-api-credits-for-max-ultra-and-team-plans), [OpenAI credits](https://developers.openai.com/api/docs/guides/chatgpt-api-credits), [Gemini billing](https://ai.google.dev/gemini-api/docs/billing).

## Research still needed

- Direct intervention evidence for Russian stress, vowel reduction, and connected speech, beyond palatalization.
- Specific acoustic features that predict listener judgments on Ember's recordings; sensitivity to devices and speech context.
- Working-memory supports during language learning and how to fade them. Current supports are reasonable design hypotheses, not a validated treatment.
- Reliable Russian speech synthesis, recognition, alignment, morphology, and stress annotation, including mathematics terminology.
- Curriculum sources and rights for specialist mathematics/philosophy passages; writing and reading interventions beyond word learning.
- Controlled evidence for LLM language tutors and evaluation methods robust to tutoring model errors.
