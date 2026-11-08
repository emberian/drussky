# Licence deputy report — 2026-11-08

The audit and deterministic credits generator are complete. **The rights/provenance gaps are
not all resolved**: credits.json deliberately marks incomplete entries instead of fabricating
contributors or pretending that the audit grants rights. No git command was run. No existing
audio, library, generator, Swift, project, or notebook file was modified.

## Files written

- `research/LICENSES.md`: findings, primary-source links, category verdicts, per-licence
  counts, all 48 text checks and 18 author records, SA/NC scope, existing UI deficiencies,
  and concrete remaining work.
- `apple/scripts/build_credits.py`: standard-library-only, offline generator; `--check`
  verifies deterministic output freshness. It reads all three sources manifests, all indices
  and both variants files, the foundation manifest, library, fonts, and literal reference keys.
- `apple/scripts/credits_evidence.json`: portable recovery of 242 English Tatoeba sentence IDs
  (230 distinct; exact Russian ID and both texts matched the original import cache), four
  verified Shtooka project credits, and the installed Silero model's pinned licence identity.
  No contributor names existed in that cache; empty contributor arrays are intentional.
- `apple/Drussky/Resources/credits.json`: one source entry per human take, grouped by kind,
  author-to-asset lists, source/licence URLs, modifications, all five full OFL texts,
  conditional ZIPA credit, Silero model credit and an honest model-authorship paragraph.
  It also records source hashes and unresolved items. `credit_status` is significant.
- `research/LICENSES-audio.csv`: all 2,775 app clips; exact per-file attribution/SA/missing
  licence flags, credited and raw authors and source links. This is the requested full list,
  not a sample or only the active index.
- `research/LICENSES-foundation.csv`: all 112 original foundation assets and their app users.
- This report.

## What must change / concrete decisions for integration

1. **Replace the current credits reader.** `CreditsView` in `ContentsView.swift` reads only
   sources.json and ignores `author`; its font sentence mentions only Denis Masharov/Marck
   Script. Render credits.json with links, source-specific changes, complete font notices
   and incomplete-entry flags. Include pairs, narrations, Tatoeba text contributors and
   reference materials. ZIPA should be shown when installed; foundation originals are
   separately scoped repository assets. Regenerate with xcodegen and check the built bundle.
   This audit-only lane did not change that UI or hand-edit the generated project.
2. **Recover Tatoeba text contributors for all 242 pairs.** Current fields identify the
   *recording speaker*, not Russian author or English translator. The generated entries retain
   both sentence URLs now but remain incomplete under the current author-credit terms. Fetch
   contributor-bearing exports/API/history, preserve the actual text licences, and populate
   evidence or, preferably, the importer/library's canonical metadata. Older Tatoeba terms
   allowed links; we did not establish that version for each contributed sentence, so do not
   use it as a universal shortcut. All affected IDs are in credits.json's unresolved list.
3. **Check the remaining 27 Commons author placeholders.** Exact filenames are below.
   Four of the original 31 were independently restored as Shtooka project credits from their
   pages; that must not become an unsupported inference for the remainder. Five foundation
   human clips have usable `credit` instead of `author`; the generator preserves those.
4. **Zamyatin `zamyatin-my-zapis-1`: stop claiming unqualified US public domain from `year: 1920`.**
   The cited Russian edition says 1952. Its relationship to the 1924 English / 1927 Russian
   publication and the selected excerpt must be established. Recommendation: use a documented
   pre-1931 version of Record 1, or exclude the current excerpt from a publicly distributed PD
   pack until checked. This is a rights hold, not a finding that the entire novel is certainly
   protected. The audit did not delete it. Belyaev is explicitly the **1925 version**, which is
   suitable; do not unnecessarily remove it because the expanded novel is later. Odoevsky's
   complete-publication basis needs the 1926 note; Ushinsky's *Blind Horse* cites a modern
   transcription without a dated edition or scans and needs edition/publication evidence.
   All author ordinary Russian terms are expired; edition questions are separate.
   Lensky's historic narration also needs first-publication evidence: 1894 is the recording
   date, and Commons' generic pre-1931 PD template does not establish the US sound-recording
   rule for recordings first published before 1923. Preserve the PD claim as a claim.
5. **Milena `e-bea15c.m4a`: remove/replace before public audio/app distribution.** Apple's
   macOS voice licence supports the requested personal non-commercial project but expressly
   excludes public sharing, even non-profit. Merely adding an Apple credit cannot cure this.
   Keep local personal use, or replace with a properly licensed recording/synthesis.
6. **Make model vs output licences explicit.** Silero v5_6_ru genuinely uses CC BY-NC-SA 4.0;
   the MIT CIS-base models are different. All 2,069 Silero outputs lack a separate licence
   field (2,044 have direct provider labels across main/pairs; 25 are recovered foundation
   syntheses). Model licensing alone does not prove every generated waveform is a derivative;
   don't relabel them all blindly, and don't advertise commercial freedom without an output
   rights basis. Commercial model use needs permission or regeneration with a suitable engine.
   Keep model hash, source revision, licence text/URL and output-rights basis separately.
7. **Document the Wiktionary/other-dictionary boundary.** Current Wiktionary is BY-SA 4.0;
   blindly applying the task's 3.0 assumption would be stale. There are 51 hand-copied reference
   table keys in the validator, including two singular-only nouns. Recommendation: retain
   those datasets under the appropriate SA terms, with exact revisions/history links and
   changes, while keeping independently written algorithms and grammatical facts separate.
   Morphology explicitly cites Wiktionary, Tikhonov, Vasmer and other dictionaries; it is not
   source-free. The linked entries in credits are reconstructed leads, not invented revision
   IDs. Record or rewrite any copied expressive notes. Citation alone isn't ShareAlike compliance.
8. **Restore drawing/test-fixture provenance.** Cursive paths appear independently authored,
   but the declared Горецкий/Федосова/Wikipedia references need edition/page or image/revision
   evidence if protected diagrams/prose were adapted. Hershey test ink has no recorded original
   package/generator/notice; recover those and retain the actual distribution acknowledgements.
   Hershey is test-only; all five production fonts already have full OFL texts. Marck Script's
   RFN is preserved, and no modification pipeline was found; upstream binary equality wasn't
   independently established.
9. **When fetching ZIPA, preserve its licence as well as its metadata.** The exact HF model
   is NC-SA 4.0; Core ML/quantized versions keep that licence. It is absent in this checkout.
   Personal use is supported; public model distribution needs attribution/SA/NC and commercial
   use requires a separate basis. Credits has a conditional ready-to-render notice.
10. **Choose the project's own publication licence.** No top-level LICENSE/COPYING was found.
    Original model-assisted material has no imported third-party content licence merely because
    models drafted it; GitHub visibility also doesn't grant a general reuse licence. Record the
    project's choice without overriding imported rights. `series.json` and `math.json` are absent
    here, so their claimed drafting process is disclosed as project-supplied context, not audited
    item-level history. PrivatePack is absent too; this report does not clear a different build's
    private resources, or research-only VoxCPM2/ESpeech outputs.

The remaining 27 Commons files (`source_page` and exact licence are in LICENSES-audio.csv):

```text
devyanosto-5cd6d5.m4a
devyatnadtsat-810460.m4a
dvenadtsat-277c83.m4a
lozhka-037c14.m4a
odinnadtsat-8074a8.m4a
okno-f663f7.m4a
otets-e28872.m4a
pravda-ce605d.m4a
privet-e24505.m4a
pyatdesyat-6cf9a2.m4a
raketa-161431.m4a
rot-7942ac.m4a
sakhar-cb9718.m4a
sdelat-e0e0b9.m4a
semdesyat-482495.m4a
semnadtsat-4997d2.m4a
sestra-3fcff1.m4a
shum-03d768.m4a
skolko-f60f4e.m4a
sledovatelno-d078df.m4a
sto-ac4909.m4a
stul-b7723c.m4a
syr-4372c8.m4a
trinadtsat-90e38d.m4a
tsirk-4bd0a7.m4a
vosem-d38592.m4a
vosemdesyat-f1afc4.m4a
```

## Checks and scope limits

`python3 apple/scripts/build_credits.py` ran successfully; a subsequent `--check` passed byte
comparison. An independent standard-library check passed: all 2,775 local M4As equal the
inventory set, exactly one audio credit owns each, all entry IDs are unique, boolean requirement
fields are uniform, all mandatory human credits contain source/licence links and processing
notes, all five OFLs include the disclaimer, all 242 translation links are preserved, all 51
reference keys are included, and all input hashes still match. Expected totals: **633 audio
attribution, 222 SA, 2,070 missing licence fields = 2,069 Silero + one Apple**. Foundation
reuse is 30 = 25 synthetic + five human. No index/variant/library reference lacks provenance.

A preliminary check correctly detected stale outputs after the generator was changed; rebuilding
resolved it. An ad-hoc validation assertion initially assumed reference group order rather than
looking up the Wiktionary ID; the corrected independent check passed. Neither was a resource defect.

No simulator/build/test suite or screenshots: no Swift or UI behavior changed, and the mission
was a file-only audit. No xcodegen execution or project edit; root owns application integration.
Free space was about 465 GiB. No caches or other sessions' files were removed or written.

Network limitations: shell HTTP had no DNS access. Several web source fetches (Tatoeba sentence
API/pages, one LibriVox page, some Commons pages and Google terms) failed. A Playwright HTTP
fallback was rejected because it required approval while the environment's approval policy is
`never`; no bypass or user permission was sought. Available primary web sources and read-only
local import caches supplied the evidence that is explicitly recorded. Therefore the audit does
not claim a complete remote revalidation or complete text-contributor recovery.

## Design disagreements and recommended next work

The current system conflates reproducibility with rights provenance. `audio_store.py` can recreate
bytes without preserving who can license them. A model licence in a provider string, a human
speaker used as a sentence author, a death date used as a US publication test, and a website name
used as a licence are each different errors. Fix the common importer/schema once rather than
adding more ad-hoc credit strings to each generator. `make_audio.py` should copy the same resolved
foundation provenance object for kind, creator and rights, including its nested fallback.

The current UI is another independent loss point: it doesn't even decode `author`. Integrate
the generated manifest first, preserve required source fields at import, then make public-pack
validation fail on genuinely unresolved attribution or prohibited distribution. Keep those rules
distinct from freshness checks and from private-device operation. SA belongs to the affected
content/adaptation; it does not automatically relicense the entire workbook application.
