# Mathematical reading should begin early, with visible support

I would open a small mathematics strand after the alphabet, alongside ordinary reading
and domestic language. Ember already knows what an injection or counterexample is;
Russian can become the unfamiliar part of a familiar argument. That is a useful
advantage, not evidence that mathematical Russian is generally easy or that its
morphology will automatically become fluent speech.

The new `math.json` makes the proposal concrete: six definitions, eight propositions
with proofs, then participles, two linked definition–theorem–proof sequences, probability,
logic and algorithmic information. The opening uses nominal definitions, genitives and
locations; later readings introduce governed cases and compressed clauses. English
answers separate comprehension from Russian production. Inference questions ask which
noun a participle modifies, which clause a quantifier governs, or which hypothesis
licenses a proof step. Knowing Euclid's theorem alone should not answer them.

The measured constraint is vocabulary. The validator reports only about 19–50% exact
course-form coverage at the recommended pages; adding earlier shelf encounters gives
roughly 36–75% on the later rungs. All remaining forms have contextual glosses, but a
gloss is support, not acquired vocabulary. These figures undercount transfer from known
lemmas, yet cognate intuition cannot repair an unknown connective or case ending.
`мно́жество`, `число́`, `то́чка`, `принадлежа́ть` and `пусть` are especially valuable
noncognate vocabulary. Calling this a universally “40% cognate” register or promising
unassisted textbook reading on a fixed month would go beyond our evidence.

I would change the course in small, testable ways:

| Existing location | Proposed change |
|---|---|
| After `u7-p4`, the complete alphabet | Offer the first definition immediately with glosses. Add a short bridge for `в`, `на`, `без`, `для`, `из`, `э́то`, `есть`, `нет`, `и`, `не`, `но`, `то́лько`. Practise `мно́жество / мно́жества / в мно́жестве` in meaningful contrasts. |
| `u16` first readings and `u21` mathematics | Bring `е́сли … то`, `ка́ждый`, `существу́ет`, `ро́вно оди́н`, `хотя́ бы`, `сле́довательно` forward into that bridge. Give the learner a choice of set/function readings before requiring the space story. |
| `u24–u25`, cases and aspect | Reuse `принадлежа́ть + Dat`, `явля́ться + Instr`, `ра́вен + Dat`, `зави́сеть от + Gen`; contrast `предполо́жим / поло́жим / полу́чим` with ongoing reasoning. Keep domestic examples as a transfer check. |
| `u27–u28`, relative clauses and participles | Pair a full `кото́рый` clause with its participial equivalent. Ask for the head noun and the governed object separately. Add adverbial participles with a shared-agent check. |
| `u29`, mathematical language | Use an unseen paragraph from Ember's chosen analysis book to choose the next vocabulary and syntax, rather than repeating already memorized proofs. |

The reading shelf should remain available before its `after_page`; that field recommends
support, not permission. Do not turn every lookup into a review obligation. A useful
trial is a few supported readings followed by an unfamiliar definition and an altered
hypothesis: can Ember explain the scope without revealing the English? Later, test the
same case frame outside mathematics. Those observations would tell us more than either
quiz completion or a vocabulary percentage alone.

There is also an immediate implementation gap: today's Library loader does not discover
`math.json`, and its decoder discards the authored teaching fields. Wire collection
loading and the existing passage/gloss machinery to these fields before evaluating the
pedagogy. A successfully decoded JSON file is not yet a usable reading experience.
