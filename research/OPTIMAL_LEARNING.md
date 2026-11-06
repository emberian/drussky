# Models for efficient Russian learning

Research update, 2026-11-05. The objective is useful, durable Russian learned with a manageable investment of time and effort. This includes understanding new sentences, producing language spontaneously, and accessing literature, science fiction, mathematics, and philosophy. Memorizing the largest number of isolated answers is only one subproblem.

Ember explicitly welcomes theoretically motivated experiments. We should build an experimental teaching policy alongside a transparent baseline. Theories can guide useful inventions before a Russian-specific trial exists; their assumptions and predicted benefits should remain inspectable.

## What optimal can mean

An optimum needs an objective, a learner model, available teaching actions, and a time horizon. Maximizing tomorrow's vocabulary recall, minimizing time to understand case marking, and acquiring comfortable conversation over months produce different policies. A mathematical guarantee is conditional on that specification.

For Rufren, a proposed finite-horizon objective is:

`expected useful competence at horizon H − λ × practice time − μ × effort`

Competence is measured on delayed and novel-context tasks, with separate listening, speaking, reading, and writing components. The weights express Ember's priorities; they are not constants furnished by a paper. Enjoyment also changes what practice is sustainable and can be a goal in its own right. The model should account for likely continued use, while avoiding optimizing engagement clicks as if they were learning.

The teacher's actions include explaining, demonstrating, asking a comprehension question, eliciting speech, giving a cue, supplying corrective feedback, selecting an excerpt, and scheduling a later revisit. Some actions both teach and diagnose. Information about the learner matters because it can improve subsequent teaching, not merely because it reduces model uncertainty.

## Evidence and theory worth implementing

| Approach | What it contributes | Scope of support | Rufren experiment |
| --- | --- | --- | --- |
| Model-based memory scheduling | Balance forgetting, strengthening, and practice cost | Vocabulary/classroom studies and conditional optimization results | Compare delayed retention per practice minute against a simple spaced baseline |
| POMDP teaching | Plan a short sequence while uncertain about the learner's understanding | Formal model plus human concept-learning experiments | Choose between explanation, contrasting examples, and diagnostic questions |
| Bayesian pedagogy | Choose examples that make the intended generalization inferable | Computational account plus bounded behavioral tasks | Teach a case/word-order distinction with a deliberately informative set |
| Interleaving and discrimination | Make the learner choose which construction applies | Direct L2 grammar experiments | Mix confusable forms while keeping instructions and context manageable |
| Adaptive challenge | Adjust difficulty as perceptual skill improves | Theoretical 85% result under specified binary-learning assumptions | Test task-local difficulty control on sound discrimination |
| Construction learning | Use frequent representative phrases as anchors for productive patterns | Naturalistic L2 corpus evidence | Establish an anchor, vary its slots, then check new-word generalization |

### O1 Memory scheduling with an explicit cost

Pavlik and Anderson (2008), *Using a Model to Compute the Optimal Schedule of Practice*, balances spacing benefits with the time cost of difficult/failed retrieval. Its abstract reports experimental benefits from model-guided scheduling. This is a stronger starting point than choosing intervals merely because they are customary. Access: indexed abstract; author-hosted full-text download timed out. [Paper](https://doi.org/10.1037/1076-898X.14.2.101).

Lindsey, Shroyer, Pashler, and Mozer (2014), *Improving Students' Long-Term Knowledge Retention Through Personalized Review*, reports benefits from personalized review in a semester-long classroom application. This supports investigating adaptation beyond a fixed schedule; it does not establish a particular scheduler for spontaneous Russian speech. Access: publisher abstract. [Paper](https://doi.org/10.1177/0956797613504302).

Tabibian et al. (2019), *Enhancing Human Learning via Spaced Repetition Optimization*, derives MEMORIZE using stochastic optimal control. Under its memory/cost assumptions, review intensity depends on predicted recall. Its Duolingo evaluation uses observational logs described as a natural experiment, not randomized deployment of the proposed policy. The dataset also uses sentence-response information as a proxy for word recall. Access: full text, model framing and evaluation methods. [Paper](https://doi.org/10.1073/pnas.1815156116).

**Our inference:** predict recall separately by modality and cue condition, and choose review effort according to usefulness and retention. Batch practice into sessions Ember actually wants; doing so changes the original continuous-time optimization problem and its guarantee. Sentence exposure should be recorded, but mere appearance of a word in a sentence should not automatically count as successful retrieval of that word.

### O2 Teaching as a partially observed planning problem

Rafferty, Brunskill, Griffiths, and Shafto (2016; online 2015), *Faster Teaching via POMDP Planning*, combines uncertainty about learner state with planning over teaching actions. Human experiments in two concept-learning tasks found acceleration relative to baselines. The work also examines consequences of different learner models. Access: full text, abstract and model/limitations sections. [Paper](https://doi.org/10.1111/cogs.12290).

**Our proposed transfer:** begin with a small set of plausible misunderstandings within one Russian construction. Plan a few steps ahead rather than modeling an entire mind. For example, compare beliefs that the first noun always names the actor versus beliefs that case endings help assign roles. A question, a worked explanation, and a contrasting pair have different teaching and diagnostic value. Include an “unmodeled difficulty” possibility so the system can request coaching instead of becoming confidently wrong.

A practical implementation can enumerate a small bank of reviewed actions and approximate a short-horizon policy. It need not begin with reinforcement learning on thousands of learners. Simulations can expose a broken planner or fragile assumptions; they cannot establish that Ember learns according to the simulated student.

### O3 Deliberately informative examples

Shafto, Goodman, and Griffiths (2014), *A Rational Account of Pedagogical Reasoning*, models teachers choosing examples and learners interpreting those choices as helpful rather than random samples. It studies bounded teaching/learning tasks; it is not a Russian grammar intervention. Access: author-hosted full text. [Paper](https://doi.org/10.1016/j.cogpsych.2013.12.004).

**Our proposed transfer:** choose example sets that distinguish likely competing rules. The constructed pair `Саша видит Машу` / `Машу видит Саша` keeps who sees whom fixed while changing order. Context must make the alternate order pragmatically sensible. A later item changes names and verbs to test whether the distinction transferred. This pair illustrates a design idea, not a complete account of Russian case or a ready lesson. Avoid teaching a misleading rule through a perfectly tidy but unrepresentative set of examples.

The tutor can explain why examples were chosen. Ember need not infer the teacher's intentions silently; explicit explanation and guided discovery can both be available actions.

### O4 Interleaving without unnecessary memory load

Nakata and Suzuki (2019), *Mixing Grammar Exercises Facilitates Long-Term Retention*, studied 115 Japanese learners of English across blocked, interleaved, and increasing practice. Interleaving produced more training errors but outperformed blocking on the one-week grammaticality-judgment test. This supports measuring delayed learning rather than rewarding only easy practice performance. Access: author-hosted full text, abstract and design. [Paper](https://doi.org/10.1111/modl.12581).

Suzuki, Yokosawa, and Aline (2022; online 2020) studied 60 English learners practicing relative clauses. Interleaving improved accuracy on immediate and delayed speaking tests; its advantage for speed was weaker. The reported benefit was not restricted to participants with high working-memory capacity. Access: publisher abstract; local full-text acquisition failed. [Paper](https://doi.org/10.1177/1362168820913985).

**Our inference:** do not equate working-memory support with avoiding interleaving. Keep task instructions, reference context, and replay controls stable while varying the grammatical distinction. Test a short introduction followed by useful contrastive practice, without assuming this sequence is always superior to interleaving from the outset.

### O5 The 85 percent rule as a local hypothesis

Wilson et al. (2019), *The Eighty Five Percent Rule for Optimal Learning*, derives approximately 84.13% correct under particular binary-classification learning assumptions, including Gaussian noise in the illustrated derivation. Demonstrations use neural learning models. It is not a human Russian trial or a universal vocabulary-retention target. Access: full text, derivation opening. [Paper](https://doi.org/10.1038/s41467-019-12552-4).

**Our proposed experiment:** use a configurable difficulty controller for a well-defined auditory discrimination task, compare delayed identification on untouched natural recordings, and examine whether useful challenge differs by contrast and session. Adjust assistance, distractor similarity, or available context carefully. A chosen practice accuracy, an SRS retention probability, lexical coverage of a text, and confidence in a pronunciation judgment are different quantities even if all are expressed as percentages.

### O6 Learning constructions through representative examples

Ellis and Ferreira-Junior (2009), *Construction Learning as a Function of Frequency, Frequency Distribution, and Function*, analyzes naturalistic acquisition of English verb–argument constructions. Frequent, prototypical exemplars appear early. Its observational corpus analysis motivates anchor-based curricula; it does not prove the fastest Russian sequence. Access: publisher abstract. [Paper](https://doi.org/10.1111/j.1540-4781.2009.00896.x).

**Our proposed transfer:** start with an understandable, useful phrase, then vary a noun, participant, location, or grammatical feature to expose the pattern. Eventually require new combinations. The model should distinguish remembering the anchor from generalizing its structure and producing it fluently.

## A curriculum that leads toward wanted texts

Build a small private corpus of passages Ember wants to read: science fiction, literature, mathematics, philosophy, and useful everyday conversations. Preserve the exact original and its edition/translation metadata. Keep generated simplifications visibly separate. Packaged reading material should have an appropriate source; imported personal passages remain personal.

Rank candidate vocabulary and constructions partly by how much wanted material they unlock, alongside broad conversational usefulness. A word's value depends on its sense, inflected forms, collocations, and surrounding syntax. Frequency alone will overvalue function words already known and may undervalue a recurring story concept. Unknown-word percentage alone does not estimate comprehension of a difficult argument.

An experimental planner could select a short sequence of lessons that covers the next passage's most important missing features, interleaving retrieval with rereading and conversation about its meaning. This resembles coverage optimization with uncertain prerequisite/transfer effects. It is our engineering hypothesis, not a demonstrated optimal-reading theorem.

## Implementation and evaluation

Keep the first baseline and experimental policy behind the same interface. Record policy version, chosen action, candidate alternatives, assumptions, assistance, and outcomes. Start with a narrow family of constructions and a modest vocabulary set; use teacher-reviewed stimuli and a coach override.

Separate structural understanding, memory strength, and production fluency. A correct explanation does not prove rapid online comprehension; a memorized sentence does not prove grammatical generalization. Keep uncertainty rather than forcing each response into one diagnosis.

Evaluate selected features with delayed, novel examples and practice time/effort. Use matched or staggered content and account for transfer between items and time trends. Occasional randomized comparisons can improve inference; the app need not make every lesson an experiment. A complex policy is worth exploring before it is validated, but claims of improvement require actual results.
