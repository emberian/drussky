# Drussky licence and attribution audit

Audited 2026-11-08 by the Codex licence deputy. Scope: this checkout's iPad application,
its bundled resources, the source published on GitHub, and the foundation audio it reuses.
This is an engineering rights audit with explicit unresolved cases, not a legal opinion.
“Clean” below means the identified use has a documented basis within the stated scope;
it is not a warranty that every upstream contributor owned everything they uploaded.

**Verdict: personal use is substantially supported, but the repository and app are not yet
ready to describe themselves as fully attributed or unrestricted for redistribution.**
The actionable holds are Tatoeba text authors, 27 remaining Commons credit placeholders,
Zamyatin's Russian edition, copied reference material, and the Apple voice's public-sharing
restriction. No explicitly ND or all-rights-reserved recording occurs in the three public
audio manifests. The uncertain items are not silently reclassified as prohibited.

## Evidence and reproducibility

Read: `apple/README.md`, `COURSE_SCHEMA.md`, `LearningCore/README.md`, every JSON inventory
named in the task, the audio generators/store, model fetcher, course/stroke/morphology/paradigm
authoring sources and validators, font licences and TTF name tables, project.yml, Swift imports,
the foundation manifest, and the pinned local Silero manifest/licence/package metadata.
Online checks use the primary sources linked below. Source pages were checked selectively,
especially ambiguous cases; this is **not** a claim that all 705 human recording permissions
were individually revalidated online. The ledger is exhaustive against the local manifests.

Run `python3 apple/scripts/build_credits.py`, then `python3 apple/scripts/build_credits.py --check`.
The latter checks deterministic freshness, not clearance. Outputs:

- [`credits.json`](../apple/Drussky/Resources/credits.json): source-by-source display strings,
  exact recorded contributor names, source/licence links, processing disclosures, per-author
  asset lists, incomplete-credit flags, font licence text and model-authorship note.
- [`LICENSES-audio.csv`](LICENSES-audio.csv): **every one of the 2,775 app clips**. Filter
  `attribution_required=True` for the complete 633-clip attribution list;
  `share_alike=True` for all 222 SA clips; `missing_license_field=True` for all 2,070 missing
  fields. `model_attribution_precaution=True` separately identifies the 2,069 Silero outputs.
  `incomplete_author=True` gives all remaining 27 uncredited human clips. The CSV retains
  filenames, manifest, original licence, credited and recorded authors, references and source URLs.
- [`LICENSES-foundation.csv`](LICENSES-foundation.csv): all 112 foundation WAVs, their rights,
  credits and app consumers; these are separate source assets, not 112 additional app clips.
- [`credits_evidence.json`](../apple/scripts/credits_evidence.json): portable, reviewed supplements.
  It preserves all 242 English Tatoeba IDs recovered from the original import cache after
  matching Russian ID and both actual texts, four verified Shtooka credits, and the pinned
  Silero manifest. It has no dependency on another checkout at generation time.

No existing provenance, course content, notebook format, audio, or application code was rewritten.
Input SHA-256 hashes in credits identify the exact source snapshot. Audio and model files being
ignored by version control does **not** exclude them from an installed app: project.yml copies
the entire Audio and Models folders. There were 2,775 local M4As and no unmanifested M4As at
inspection; ZIPA was absent (`Models/.gitkeep` only). `series.json`, `math.json` and PrivatePack
content were absent. An actual release archive still needs its own bundle check.

## Audio

### Counts of the actual `license` field

These are stored spellings, not guesses from provider strings. CC0 and CC0 1.0 are normalized
to CC0-1.0 in generated entries but kept separate here for reconciliation.

| Recorded licence | sources.json | sources-pairs.json | sources-narration.json | Total |
|---|---:|---:|---:|---:|
| CC BY 2.0 fr | 144 | 19 | 0 | 163 |
| CC BY 2.5 | 1 | 0 | 0 | 1 |
| CC BY 3.0 | 4 | 0 | 1 | 5 |
| CC BY 3.0 us | 32 | 1 | 0 | 33 |
| CC BY 4.0 | 208 | 1 | 0 | 209 |
| CC BY-SA 2.5 | 10 | 4 | 0 | 14 |
| CC BY-SA 3.0 | 58 | 9 | 0 | 67 |
| CC BY-SA 4.0 | 104 | 36 | 1 | 141 |
| CC0 | 22 | 3 | 0 | 25 |
| CC0 1.0 | 40 | 0 | 0 | 40 |
| Public domain | 2 | 0 | 5 | 7 |
| Missing | 1,958 | 112 | 0 | 2,070 |
| **Total** | **2,583** | **185** | **7** | **2,775** |

Kinds: 705 human recordings, 2,069 AI-generated recordings, one macOS TTS recording.
The main manifest has 625 human + 1,957 synthetic + one macOS; pairs has 73 human + 112
synthetic. `index.json` has 2,443 keys, `index-pairs.json` 75 and `index-narration.json` seven.
The generator also reads both variants manifests and library audio references. These are
references to the inventory, not additional recordings. Count all manifest assets, including
alternate takes, because the folder is bundled whole.

### Human pronunciation recordings — needs attribution

Wikimedia Commons hosts Shtooka, Lingua Libre, individual recordings and some material whose
source is Forvo. Tatoeba supplies sentence recordings. Hosting site, speaker, recorder,
uploader, literary author and copyright licensor are different roles; don't merge them.
`credit` strings and named speakers/recorders are retained verbatim in credits. No universal
magic attribution sentence is mandated: our assembled strings carry title, credited people,
source, exact licence version/jurisdiction and the changes, plus any supplied credit statement.

For CC BY and BY-SA retain the supplied attribution and notices, link to the original and
licence, and describe the trim/normalization/AAC conversion and any excerpt. Earlier versions'
title requirements are served by the recorded title, or the known word where no title exists.
Do not imply endorsement. Use the French licence for `CC BY 2.0 fr`, and the US port for
`CC BY 3.0 us`; substituting an unported or later licence loses information. See the
[CC BY 2.0 FR legal text](https://creativecommons.org/licenses/by/2.0/fr/legalcode),
[CC BY 3.0 US](https://creativecommons.org/licenses/by/3.0/us/),
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) and
[CC BY-SA 4.0 legal text](https://creativecommons.org/licenses/by-sa/4.0/legalcode.en).

The **222 BY-SA recordings keep their licences**. An edited adaptation must be offered under
the applicable same/compatible SA licence; use the recorded version unless a deliberate upgrade
is documented. Mere format conversion is not automatically an adaptation (4.0 §2(a)(4)).
Cuts and creative edits deserve a change notice and conservative treatment as adaptations.
Putting separate clips beside application code is ordinarily a collection, not a requirement
to license the Swift app under CC BY-SA. Do not impose additional restrictions or DRM on the
licensed recordings; preserve a usable way to obtain/reuse those assets when distributing.
Credit alone does not satisfy ShareAlike. [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/legalcode)
also distinguishes a collection from an adaptation.

Forvo's general site conditions are not a substitute for the selected Commons file's licence.
For example, [Ru-оно.ogg](https://commons.wikimedia.org/wiki/File:Ru-оно.ogg) identifies
**1640max**, Forvo as source, and CC BY-SA 4.0 with explicit attribution to 1640max.
Use that documented permission, retain the file-page evidence, and do not assume other Forvo
downloads have it. This audit found no provenance field recording the original permission
review/VRT ticket, if one exists; targeted verification is still appropriate before broad release.

**Missing author details:** 31 main-manifest records say `not stated; see source page`.
Four pages were independently checked and name The Shtooka Project as the provider:
[десять](https://commons.wikimedia.org/wiki/File:Ru-десять.ogg),
[двадцать](https://commons.wikimedia.org/wiki/File:Ru-двадцать.ogg),
[неделя](https://commons.wikimedia.org/wiki/File:Ru-неделя.ogg),
[один](https://commons.wikimedia.org/wiki/File:Ru-один.ogg).
The generator supplements those four, explicitly preserving the raw author and evidence.
The other **27 remain incomplete** in the CSV and credits; do not invent individual names or
infer Shtooka from a filename alone. Five reused human foundation clips have no `author` but
do carry usable `credit` statements; credits uses these without pretending an author field exists.
Shtooka clips with only a project credit can credit that designated organization where the
source supplies no individual; an absent individual is not automatically an infringement.

**CC0 / PD — clean on the recorded permission where its basis is established; see narration exceptions below.** There are 65 CC0
recordings and seven labelled public domain. One useful discrepancy: the foundation manifest
labels Ru-а.ogg CC BY 2.0 FR while the app labels its take public domain. The
[current file page](https://commons.wikimedia.org/wiki/File:Ru-а.ogg) displays **both** grants;
this is not proof of an illegal relabelling. Preserve which permission was selected and its
page revision. Public-domain sound recordings require their own basis, not just a dead author.

### Narrations — mixed licences; credit readers separately

| Text / clip suffix after `narr-` | Reader | Recording rights / verdict |
|---|---|---|
| chekhov-tolstyi-i-tonkii | Pogrebnoj-Alexandroff | CC BY 3.0; **needs attribution** |
| dostoevsky-zapiski-iz-podpolya-2x2 | Yakovlev Valery | LibriVox PD in USA; **clean for US use**, courtesy credit |
| pushkin-stantsionnyi-smotritel | Xenium5 | LibriVox PD in USA; **clean for US use**, courtesy credit |
| pushkin-vystrel | Xenium5 | LibriVox PD in USA; **clean for US use**, courtesy credit |
| pushkin-zimnee-utro | Шантопик | CC BY-SA 4.0; **needs attribution and retained SA** |
| tolstoy-muravei-i-golubka | Luiza Karimova | LibriVox PD in USA; **clean for US use**, courtesy credit |
| turgenev-russkii-yazyk | А. П. Ленский (1894) | Historic phonograph; Commons labels PD; **needs a decision on US first-publication evidence** |

The machine credits contain every source page, filename, reader and exact recorded processing
interval. `sources-narration.json` is the operative record: its cut offsets differ from the
older table in `NARRATION_ALIGNMENT.md`. The four LibriVox items come from
[Belkin Tales](https://librivox.org/belkin-tales-by-alexander-pushkin/),
[Notes from Underground](https://librivox.org/zapiski-iz-podpolya-by-fyodor-dostoevsky/), and
[Multilingual Short Works 012](https://librivox.org/multilingual-short-works-collection-012-by-various/).
The Notes page could not be re-fetched in this environment; its reader is taken from the local
provenance. LibriVox explicitly limits its PD assurance to the USA. Russian-language subject
matter does not make a modern performance PD in Russia: performer/phonogram rights and the
effect of a dedication abroad need separate consideration. **Needs a decision for redistribution
in jurisdictions outside that assurance.** [LibriVox policy](https://librivox.org/pages/public-domain/).

For Lensky, the [Commons source page](https://commons.wikimedia.org/wiki/File:И._А._Тургенев._Русский_язык_(чит._А._П._Ленский).ogg)
dates the reading to 30 March 1894, cites vk.com and applies a general PD-old-70-expired
template. That is evidence of recording date, not a documented first publication. US recordings
**first published before 1923** have completed their special federal term; the generic Commons
pre-1931 publication template is not sufficient to establish that sound-recording condition.
Keep the recorded PD claim and courtesy reader credit, but obtain first-publication evidence
before treating this as independently cleared for US redistribution. No independently creative
modern restoration was identified. Turgenev's 1883 death alone cannot clear the recording.
[US sound-recording law, §1401](https://www.copyright.gov/title17/92chap14.html).

### Silero — personal non-commercial model use clean; output/distribution decision required

The actual model is **v5_6_ru**, not a generic “Silero v5”: SHA-256
`04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e`, source revision
`e382298387332f2bf461fe6eadfde99104611b0c`. The installed pinned LICENSE is **CC BY-NC-SA 4.0**
(SHA-256 `1349a4b6148492b44f629e64eed676612e234fe9a839e4f3b277c1482c8849f1`).
That agrees with the recorded provider label. The
[upstream legal text](https://raw.githubusercontent.com/snakers4/silero-models/master/LICENSE)
has SA even though the README abbreviates it loosely as CC-NC-BY. The
[README](https://github.com/snakers4/silero-models#licence) explicitly excepts the **base CIS**
models under MIT; that exception is not v5_6_ru. Pin both model and terms when changing engines.

None of the 2,069 outputs has a separate `license` field. Main sources has 1,932 explicit
Silero provider labels and 25 reused synthetic foundation clips whose model can be recovered
from the foundation manifest; pairs supplies the other 112. These are **missing output-licence
metadata, not 2,069 unknown model identities**. Credits separates the known model licence
from the unestablished output licence, and gives the Silero Team credit with processing details.

Non-commercial personal inference is within the model permission. The
[NC-SA legal code](https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode.en) governs
licensed material and adaptations; it does not by itself establish that every generated waveform
is copyrightable or contains protected model expression. Do not mechanically label every output
CC BY-NC-SA, and do not promise unrestricted commercial output rights either. For future
commercial inference/distribution obtain applicable commercial terms or switch to an adequately
licensed model and regenerate. For non-personal non-commercial sharing, document the output-rights
basis and retain this credit; if output is treated as adapted licensed material, NC/SA obligations
follow. Distributing converted/modified model weights clearly needs the model's attribution,
NC and SA conditions. Silero weights and PyTorch are not app resources in this checkout.

### Apple voice — clean for the stated private project; remove from public audio distributions

`e-bea15c.m4a` (key е) uses `say -v Milena`. Its metadata lacks a licence field but names the
provider, voice, synthesis command and processing. [macOS Tahoe SLA §2F](https://www.apple.com/legal/sla/docs/macOSTahoe.pdf)
allows personal non-commercial projects and excludes public sharing, including non-profit
redistribution. **A free download is not enough.** Keep this take personal or replace it before
sharing the audio/app. Source code invoking an installed system voice does not ship Apple's
voice model, but exporting its recorded output is a separate question. The original macOS
version used for generation is not recorded; the contemporary SLA supplies the conservative
boundary. An attribution does not grant redistribution rights.

### Foundation and audio_store.py — provenance repairs needed

`make_audio.py` references `crates/rufren-app/assets/foundations` and the foundation text manifest.
The 112 foundation assets are 85 Silero and 27 human recordings. Normalizing case only, the
human licence totals are CC BY 2.0 FR **11**, CC BY 3.0 US **6**, CC BY-SA 3.0 **6**,
CC BY-SA 4.0 **3**, CC BY-SA 2.5 **1**. All 85 synthetic entries lack a licence field but
identify local_silero_tts and the pinned model. Thirty foundation assets are reused in the
app: 25 synthetic, five human. Source repository distribution of the other WAVs is still a
distribution to audit; credits includes their 27 human credits as a separately scoped group.

`audio_store.restore_fields` preserves original path, SHA-1, URL or synthesis recipe and pipeline;
this is good reconstruction provenance but not a rights schema. `make_audio.py`'s foundation
path drops human `author`, and drops synthetic provider/model/voice/rights information. It
also reads kind with a `source_response.provenance` fallback while building its credits map
from top-level `provenance` only. These should use one shared provenance copier/validator.
Add separate source/model/output licence fields, licence URL, creator/reader/recorder roles,
permission/page revision and acquisition date, plus modification notes. Preserve required
attribution URLs, copyright notices and disclaimers when supplied. Hashes alone prove bytes,
not permission. Do not “repair” shared sources while other audio agents are appending.

## Texts

### Public-domain test and author dates

For Russian economic rights, [Civil Code §1281](https://www.wipo.int/wipolex/en/text/440579)
starts the ordinary 70-year period on the January following death (last surviving co-author
for a joint work). Wartime work/participation can add four years; posthumous rehabilitation
and certain first posthumous publications have special rules. Name/authorship/integrity rights
do not simply disappear with economic rights. All named authors here died by 1942; the ordinary
terms, even Belyaev's possible four-year addition through 2016, have elapsed. No later
rehabilitation-based term was established for these selected works. A modern edition's creative
editorial material or translation needs its own analysis. [Russian Code §§1281–1282](https://www.wipo.int/wipolex/en/text/440579).

US rights are **not decided by Russian death+70 alone**. As of 2026, works published through
1930 have completed even a 95-year publication term. URAA can restore eligible foreign works
that remained protected in their source country on the restoration date; it doesn't extend
the maximum US term. Later/unpublished versions need first-publication, edition and sometimes
renewal analysis. [Copyright Office, URAA](https://www.copyright.gov/circs/circ38b.pdf),
[duration](https://www.copyright.gov/circs/circ15a.pdf).

The table below checks every actual text ID; author dates are linked to Wikisource author records
in the following author register. “Clean” is for the original Russian literary text, excluding
modern commentary, recordings and translations. Metadata years are not all first-publication years.

| Text ID (source edition) | Author death | Recorded year | Russian term / US verdict |
|---|---:|---:|---|
| [tolstoy-lev-i-mysh](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-otets-i-synovya](https://ru.wikisource.org/wiki/%D0%92%D1%82%D0%BE%D1%80%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-lgun](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-muravei-i-golubka](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-kostochka](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-staryi-ded-i-vnuchek](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-devochka-i-griby](https://ru.wikisource.org/wiki/%D0%92%D1%82%D0%BE%D1%80%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-kotenok](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-filipok](https://ru.wikisource.org/wiki/%D0%A4%D0%B8%D0%BB%D0%B8%D0%BF%D0%BE%D0%BA_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-akula](https://ru.wikisource.org/wiki/%D0%92%D1%82%D0%BE%D1%80%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-lev-i-sobachka](https://ru.wikisource.org/wiki/%D0%92%D1%82%D0%BE%D1%80%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-pozharnye-sobaki](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-kuda-devaetsya-voda](https://ru.wikisource.org/wiki/%D0%9F%D0%B5%D1%80%D0%B2%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-otchego-veter](https://ru.wikisource.org/wiki/%D0%92%D1%82%D0%BE%D1%80%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-magnit](https://ru.wikisource.org/wiki/%D0%92%D1%82%D0%BE%D1%80%D0%B0%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [tolstoy-led-voda-par](https://ru.wikisource.org/wiki/%D0%A2%D1%80%D0%B5%D1%82%D1%8C%D1%8F_%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B0%D1%8F_%D0%BA%D0%BD%D0%B8%D0%B3%D0%B0_%D0%B4%D0%BB%D1%8F_%D1%87%D1%82%D0%B5%D0%BD%D0%B8%D1%8F_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)) | 1910 | 1875 | Expired / clean, pre-1931 publication |
| [ushinsky-chetyre-zhelaniya](https://ru.wikisource.org/wiki/%D0%A7%D0%B5%D1%82%D1%8B%D1%80%D0%B5_%D0%B6%D0%B5%D0%BB%D0%B0%D0%BD%D0%B8%D1%8F_(%D0%A3%D1%88%D0%B8%D0%BD%D1%81%D0%BA%D0%B8%D0%B9)) | 1871 | 1864 | Expired / clean, pre-1931 publication |
| [ushinsky-slepaya-loshad](https://ru.wikisource.org/wiki/%D0%A1%D0%BB%D0%B5%D0%BF%D0%B0%D1%8F_%D0%BB%D0%BE%D1%88%D0%B0%D0%B4%D1%8C_(%D0%A3%D1%88%D0%B8%D0%BD%D1%81%D0%BA%D0%B8%D0%B9)) | 1871 | absent | Ordinary author term expired / **needs edition and publication evidence** |
| [krylov-vorona-i-lisitsa](https://ru.wikisource.org/wiki/%D0%92%D0%BE%D1%80%D0%BE%D0%BD%D0%B0_%D0%B8_%D0%9B%D0%B8%D1%81%D0%B8%D1%86%D0%B0_(%D0%9A%D1%80%D1%8B%D0%BB%D0%BE%D0%B2)) | 1844 | 1808 | Expired / clean, pre-1931 publication |
| [krylov-strekoza-i-muravei](https://ru.wikisource.org/wiki/%D0%A1%D1%82%D1%80%D0%B5%D0%BA%D0%BE%D0%B7%D0%B0_%D0%B8_%D0%9C%D1%83%D1%80%D0%B0%D0%B2%D0%B5%D0%B9_(%D0%9A%D1%80%D1%8B%D0%BB%D0%BE%D0%B2)) | 1844 | 1808 | Expired / clean, pre-1931 publication |
| [krylov-lebed-shchuka-i-rak](https://ru.wikisource.org/wiki/%D0%9B%D0%B5%D0%B1%D0%B5%D0%B4%D1%8C,_%D0%A9%D1%83%D0%BA%D0%B0_%D0%B8_%D0%A0%D0%B0%D0%BA_(%D0%9A%D1%80%D1%8B%D0%BB%D0%BE%D0%B2)) | 1844 | 1816 | Expired / clean, pre-1931 publication |
| [krylov-martyshka-i-ochki](https://ru.wikisource.org/wiki/%D0%9C%D0%B0%D1%80%D1%82%D1%8B%D1%88%D0%BA%D0%B0_%D0%B8_%D0%BE%D1%87%D0%BA%D0%B8_(%D0%9A%D1%80%D1%8B%D0%BB%D0%BE%D0%B2)) | 1844 | 1815 | Expired / clean, pre-1931 publication |
| [pushkin-zimnee-utro](https://ru.wikisource.org/wiki/%D0%97%D0%B8%D0%BC%D0%BD%D0%B5%D0%B5_%D1%83%D1%82%D1%80%D0%BE_(%D0%9F%D1%83%D1%88%D0%BA%D0%B8%D0%BD)) | 1837 | 1830 | Expired / clean, pre-1931 publication |
| [turgenev-vorobei](https://ru.wikisource.org/wiki/%D0%92%D0%BE%D1%80%D0%BE%D0%B1%D0%B5%D0%B9_(%D0%A2%D1%83%D1%80%D0%B3%D0%B5%D0%BD%D0%B5%D0%B2)) | 1883 | 1878 | Expired / clean, pre-1931 publication |
| [turgenev-russkii-yazyk](https://ru.wikisource.org/wiki/%D0%A0%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9_%D1%8F%D0%B7%D1%8B%D0%BA_(%D0%A2%D1%83%D1%80%D0%B3%D0%B5%D0%BD%D0%B5%D0%B2)) | 1883 | 1882 | Expired / clean, pre-1931 publication |
| [chekhov-tolstyi-i-tonkii](https://ru.wikisource.org/wiki/%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D1%8B%D0%B9_%D0%B8_%D1%82%D0%BE%D0%BD%D0%BA%D0%B8%D0%B9_(%D0%A7%D0%B5%D1%85%D0%BE%D0%B2)/%D0%9F%D0%A1%D0%A1_1975) | 1904 | 1883 | Expired / clean, pre-1931 publication |
| [chekhov-khameleon](https://ru.wikisource.org/wiki/%D0%A5%D0%B0%D0%BC%D0%B5%D0%BB%D0%B5%D0%BE%D0%BD_(%D0%A7%D0%B5%D1%85%D0%BE%D0%B2)/%D0%92%D0%B0%D1%80%D0%B8%D0%B0%D0%BD%D1%82_%D1%81%D0%B1%D0%BE%D1%80%D0%BD%D0%B8%D0%BA%D0%B0_%C2%AB%D0%9F%D1%91%D1%81%D1%82%D1%80%D1%8B%D0%B5_%D1%80%D0%B0%D1%81%D1%81%D0%BA%D0%B0%D0%B7%D1%8B%C2%BB) | 1904 | 1884 | Expired / clean, pre-1931 publication |
| [chekhov-smert-chinovnika](https://ru.wikisource.org/wiki/%D0%A1%D0%BC%D0%B5%D1%80%D1%82%D1%8C_%D1%87%D0%B8%D0%BD%D0%BE%D0%B2%D0%BD%D0%B8%D0%BA%D0%B0_(%D0%A7%D0%B5%D1%85%D0%BE%D0%B2)) | 1904 | 1883 | Expired / clean, pre-1931 publication |
| [chekhov-repetitor](https://ru.wikisource.org/wiki/%D0%A0%D0%B5%D0%BF%D0%B5%D1%82%D0%B8%D1%82%D0%BE%D1%80_(%D0%A7%D0%B5%D1%85%D0%BE%D0%B2)) | 1904 | 1884 | Expired / clean, pre-1931 publication |
| [chekhov-kashtanka](https://ru.wikisource.org/wiki/%D0%9A%D0%B0%D1%88%D1%82%D0%B0%D0%BD%D0%BA%D0%B0_(%D0%A7%D0%B5%D1%85%D0%BE%D0%B2)) | 1904 | 1887 | Expired / clean, pre-1931 publication |
| [pushkin-skazka-o-rybake](https://ru.wikisource.org/wiki/%D0%A1%D0%BA%D0%B0%D0%B7%D0%BA%D0%B0_%D0%BE_%D1%80%D1%8B%D0%B1%D0%B0%D0%BA%D0%B5_%D0%B8_%D1%80%D1%8B%D0%B1%D0%BA%D0%B5_(%D0%9F%D1%83%D1%88%D0%BA%D0%B8%D0%BD)) | 1837 | 1835 | Expired / clean, pre-1931 publication |
| [pushkin-stantsionnyi-smotritel](https://ru.wikisource.org/wiki/%D0%9F%D0%BE%D0%B2%D0%B5%D1%81%D1%82%D0%B8_%D0%BF%D0%BE%D0%BA%D0%BE%D0%B9%D0%BD%D0%BE%D0%B3%D0%BE_%D0%98%D0%B2%D0%B0%D0%BD%D0%B0_%D0%9F%D0%B5%D1%82%D1%80%D0%BE%D0%B2%D0%B8%D1%87%D0%B0_%D0%91%D0%B5%D0%BB%D0%BA%D0%B8%D0%BD%D0%B0_(%D0%9F%D1%83%D1%88%D0%BA%D0%B8%D0%BD)/%D0%A1%D1%82%D0%B0%D0%BD%D1%86%D0%B8%D0%BE%D0%BD%D0%BD%D1%8B%D0%B9_%D1%81%D0%BC%D0%BE%D1%82%D1%80%D0%B8%D1%82%D0%B5%D0%BB%D1%8C) | 1837 | 1831 | Expired / clean, pre-1931 publication |
| [pushkin-vystrel](https://ru.wikisource.org/wiki/%D0%9F%D0%BE%D0%B2%D0%B5%D1%81%D1%82%D0%B8_%D0%BF%D0%BE%D0%BA%D0%BE%D0%B9%D0%BD%D0%BE%D0%B3%D0%BE_%D0%98%D0%B2%D0%B0%D0%BD%D0%B0_%D0%9F%D0%B5%D1%82%D1%80%D0%BE%D0%B2%D0%B8%D1%87%D0%B0_%D0%91%D0%B5%D0%BB%D0%BA%D0%B8%D0%BD%D0%B0_(%D0%9F%D1%83%D1%88%D0%BA%D0%B8%D0%BD)/%D0%92%D1%8B%D1%81%D1%82%D1%80%D0%B5%D0%BB) | 1837 | 1831 | Expired / clean, pre-1931 publication |
| [chekhov-pismo-k-uchenomu-sosedu](https://ru.wikisource.org/wiki/%D0%9F%D0%B8%D1%81%D1%8C%D0%BC%D0%BE_%D0%BA_%D1%83%D1%87%D1%91%D0%BD%D0%BE%D0%BC%D1%83_%D1%81%D0%BE%D1%81%D0%B5%D0%B4%D1%83_(%D0%A7%D0%B5%D1%85%D0%BE%D0%B2)) | 1904 | 1880 | Expired / clean, pre-1931 publication |
| [tsiolkovsky-na-lune](https://ru.wikisource.org/wiki/%D0%9D%D0%B0_%D0%9B%D1%83%D0%BD%D0%B5_(%D0%A6%D0%B8%D0%BE%D0%BB%D0%BA%D0%BE%D0%B2%D1%81%D0%BA%D0%B8%D0%B9)) | 1935 | 1893 | Expired / clean, pre-1931 publication |
| [bogdanov-krasnaya-zvezda](https://ru.wikisource.org/wiki/%D0%9A%D1%80%D0%B0%D1%81%D0%BD%D0%B0%D1%8F_%D0%B7%D0%B2%D0%B5%D0%B7%D0%B4%D0%B0_(%D0%91%D0%BE%D0%B3%D0%B4%D0%B0%D0%BD%D0%BE%D0%B2)/%D0%AD%D1%82%D0%B5%D1%80%D0%BE%D0%BD%D0%B5%D1%84) | 1928 | 1908 | Expired / clean, pre-1931 publication |
| [belyaev-golova-douelya](https://ru.wikisource.org/wiki/%D0%93%D0%BE%D0%BB%D0%BE%D0%B2%D0%B0_%D0%BF%D1%80%D0%BE%D1%84%D0%B5%D1%81%D1%81%D0%BE%D1%80%D0%B0_%D0%94%D0%BE%D1%83%D1%8D%D0%BB%D1%8F_(%D0%91%D0%B5%D0%BB%D1%8F%D0%B5%D0%B2)/1925_(%D0%A1%D0%9E)) | 1942 | 1925 | Expired even with +4 years / clean, **1925 version** |
| [kuprin-zhidkoe-solntse](https://ru.wikisource.org/wiki/%D0%96%D0%B8%D0%B4%D0%BA%D0%BE%D0%B5_%D1%81%D0%BE%D0%BB%D0%BD%D1%86%D0%B5_(%D0%9A%D1%83%D0%BF%D1%80%D0%B8%D0%BD)) | 1938 | 1913 | Expired / clean, pre-1931 publication |
| [kovalevskaya-oboi](https://ru.wikisource.org/wiki/%D0%92%D0%BE%D1%81%D0%BF%D0%BE%D0%BC%D0%B8%D0%BD%D0%B0%D0%BD%D0%B8%D1%8F_%D0%B4%D0%B5%D1%82%D1%81%D1%82%D0%B2%D0%B0_(%D0%9A%D0%BE%D0%B2%D0%B0%D0%BB%D0%B5%D0%B2%D1%81%D0%BA%D0%B0%D1%8F)) | 1891 | 1890 | Expired / clean, pre-1931 publication |
| [prutkov-plody-razdumya](https://ru.wikisource.org/wiki/%D0%9F%D0%BB%D0%BE%D0%B4%D1%8B_%D1%80%D0%B0%D0%B7%D0%B4%D1%83%D0%BC%D1%8C%D1%8F_(%D0%9F%D1%80%D1%83%D1%82%D0%BA%D0%BE%D0%B2)) | 1908 | 1854 | Expired / clean, pre-1931 publication |
| [zamyatin-my-zapis-1](https://ru.wikisource.org/wiki/%D0%9C%D1%8B_(%D0%97%D0%B0%D0%BC%D1%8F%D1%82%D0%B8%D0%BD)/%D0%97%D0%B0%D0%BF%D0%B8%D1%81%D1%8C_1-%D0%B0%D1%8F) | 1937 | 1920 | Ordinary term expired / **needs a decision**, 1952 edition |
| [bryusov-respublika-yuzhnogo-kresta](https://ru.wikisource.org/wiki/%D0%A0%D0%B5%D1%81%D0%BF%D1%83%D0%B1%D0%BB%D0%B8%D0%BA%D0%B0_%D0%AE%D0%B6%D0%BD%D0%BE%D0%B3%D0%BE_%D0%9A%D1%80%D0%B5%D1%81%D1%82%D0%B0_(%D0%91%D1%80%D1%8E%D1%81%D0%BE%D0%B2)/1911_(%D0%92%D0%A2)) | 1924 | 1907 | Expired / clean, pre-1931 publication |
| [odoevsky-4338-god](https://ru.wikisource.org/wiki/4338-%D0%B9_%D0%B3%D0%BE%D0%B4_(%D0%9E%D0%B4%D0%BE%D0%B5%D0%B2%D1%81%D0%BA%D0%B8%D0%B9)) | 1869 | 1835 | Expired / clean through the **1926** publication; correct year basis |
| [tolstoy-voina-i-mir-ischislenie](https://ru.wikisource.org/wiki/%D0%92%D0%BE%D0%B9%D0%BD%D0%B0_%D0%B8_%D0%BC%D0%B8%D1%80_(%D0%A2%D0%BE%D0%BB%D1%81%D1%82%D0%BE%D0%B9)/%D0%A2%D0%BE%D0%BC_3) | 1910 | 1869 | Expired / clean, pre-1931 publication |
| [dostoevsky-zapiski-iz-podpolya-2x2](https://ru.wikisource.org/wiki/%D0%97%D0%B0%D0%BF%D0%B8%D1%81%D0%BA%D0%B8_%D0%B8%D0%B7_%D0%BF%D0%BE%D0%B4%D0%BF%D0%BE%D0%BB%D1%8C%D1%8F_(%D0%94%D0%BE%D1%81%D1%82%D0%BE%D0%B5%D0%B2%D1%81%D0%BA%D0%B8%D0%B9)/%D0%A7%D0%B0%D1%81%D1%82%D1%8C_1/%D0%93%D0%BB%D0%B0%D0%B2%D0%B0_9) | 1881 | 1864 | Expired / clean, pre-1931 publication |
| [dostoevsky-son-smeshnogo-cheloveka](https://ru.wikisource.org/wiki/%D0%A1%D0%BE%D0%BD_%D1%81%D0%BC%D0%B5%D1%88%D0%BD%D0%BE%D0%B3%D0%BE_%D1%87%D0%B5%D0%BB%D0%BE%D0%B2%D0%B5%D0%BA%D0%B0_(%D0%94%D0%BE%D1%81%D1%82%D0%BE%D0%B5%D0%B2%D1%81%D0%BA%D0%B8%D0%B9)) | 1881 | 1877 | Expired / clean, pre-1931 publication |
| [lomonosov-vechernee-razmyshlenie](https://ru.wikisource.org/wiki/%D0%92%D0%B5%D1%87%D0%B5%D1%80%D0%BD%D0%B5%D0%B5_%D1%80%D0%B0%D0%B7%D0%BC%D1%8B%D1%88%D0%BB%D0%B5%D0%BD%D0%B8%D0%B5_%D0%BE_%D0%91%D0%BE%D0%B6%D0%B8%D0%B5%D0%BC_%D0%B2%D0%B5%D0%BB%D0%B8%D1%87%D0%B5%D1%81%D1%82%D0%B2%D0%B5_%D0%BF%D1%80%D0%B8_%D1%81%D0%BB%D1%83%D1%87%D0%B0%D0%B5_%D0%B2%D0%B5%D0%BB%D0%B8%D0%BA%D0%BE%D0%B3%D0%BE_%D1%81%D0%B5%D0%B2%D0%B5%D1%80%D0%BD%D0%BE%D0%B3%D0%BE_%D1%81%D0%B8%D1%8F%D0%BD%D0%B8%D1%8F_(%D0%9B%D0%BE%D0%BC%D0%BE%D0%BD%D0%BE%D1%81%D0%BE%D0%B2)) | 1765 | 1748 | Expired / clean, pre-1931 publication |
| [solovyov-smysl-lyubvi](https://ru.wikisource.org/wiki/%D0%A1%D0%BC%D1%8B%D1%81%D0%BB_%D0%BB%D1%8E%D0%B1%D0%B2%D0%B8_(%D0%A1%D0%BE%D0%BB%D0%BE%D0%B2%D1%8C%D0%B5%D0%B2)) | 1900 | 1892 | Expired / clean, pre-1931 publication |

Author register (death years; modern-calendar 1871 for Ushinsky, also reported as 1870 Old Style):

| Author record | Death | Texts |
|---|---:|---:|
| [Лев Толстой](https://ru.wikisource.org/wiki/Автор:Лев_Николаевич_Толстой) | 1910 | 17 |
| [Константин Ушинский](https://ru.wikisource.org/wiki/Автор:Константин_Дмитриевич_Ушинский) | 1871 | 2 |
| [Иван Крылов](https://ru.wikisource.org/wiki/Автор:Иван_Андреевич_Крылов) | 1844 | 4 |
| [Александр Пушкин](https://ru.wikisource.org/wiki/Автор:Александр_Сергеевич_Пушкин) | 1837 | 4 |
| [Иван Тургенев](https://ru.wikisource.org/wiki/Автор:Иван_Сергеевич_Тургенев) | 1883 | 2 |
| [Антон Чехов](https://ru.wikisource.org/wiki/Автор:Антон_Павлович_Чехов) | 1904 | 6 |
| [Константин Циолковский](https://ru.wikisource.org/wiki/Автор:Константин_Эдуардович_Циолковский) | 1935 | 1 |
| [Александр Богданов](https://ru.wikisource.org/wiki/Автор:Александр_Александрович_Богданов) | 1928 | 1 |
| [Александр Беляев](https://ru.wikisource.org/wiki/Автор:Александр_Романович_Беляев) | 1942 | 1 |
| [Александр Куприн](https://ru.wikisource.org/wiki/Автор:Александр_Иванович_Куприн) | 1938 | 1 |
| [Софья Ковалевская](https://ru.wikisource.org/wiki/Автор:Софья_Васильевна_Ковалевская) | 1891 | 1 |
| [Козьма Прутков](https://ru.wikisource.org/wiki/Автор:Козьма_Прутков) | 1908 | 1 |
| [Евгений Замятин](https://ru.wikisource.org/wiki/Автор:Евгений_Иванович_Замятин) | 1937 | 1 |
| [Валерий Брюсов](https://ru.wikisource.org/wiki/Автор:Валерий_Яковлевич_Брюсов) | 1924 | 1 |
| [Владимир Одоевский](https://ru.wikisource.org/wiki/Автор:Владимир_Фёдорович_Одоевский) | 1869 | 1 |
| [Фёдор Достоевский](https://ru.wikisource.org/wiki/Автор:Фёдор_Михайлович_Достоевский) | 1881 | 2 |
| [Михаил Ломоносов](https://ru.wikisource.org/wiki/Автор:Михаил_Васильевич_Ломоносов) | 1765 | 1 |
| [Владимир Соловьёв](https://ru.wikisource.org/wiki/Автор:Владимир_Сергеевич_Соловьёв) | 1900 | 1 |

Козьма Прутков is a collective pseudonym: A. K. Tolstoy died 1875, Vladimir Zhemchuzhnikov 1884,
Alexander Zhemchuzhnikov 1896 and Alexey Zhemchuzhnikov 1908. Use the last real co-author, not
the fictional persona’s biography. The ordinary life+70 comparison is expired for every author
in this register; US edition checks remain separate.

**Zamyatin — needs a decision, exclude from an asserted-PD public pack until resolved.**
`zamyatin-my-zapis-1` says `year: 1920`, but that is not a publication date. The
[cited Russian edition](https://ru.wikisource.org/wiki/Мы_(Замятин)/Запись_1-ая)
and [parent page](https://ru.wikisource.org/wiki/Мы_(Замятин)) say created 1921, published 1952.
The library's own licence note mentions Prague 1927 and New York 1952. The 1924 English
translation and earlier Russian publication may establish an earlier publication of some or
all underlying expression, but the audit has not collated this excerpt against them or
established authorization/US status of the later edition. Do not assert either “definitely
protected until 2048” or “definitely PD because written in 1920.” Russian ordinary term ended
after 2007; the unresolved issue is the US edition/publication history. Root should compare
Record 1 against a documented pre-1931 publication, use that version, or omit this excerpt
from the public pack pending clearance. Personal non-commercial use is not blanket US fair use.

**Belyaev — clean for this 1925 version.** The source URL explicitly selects
[1925 (СО)](https://ru.wikisource.org/wiki/Голова_профессора_Доуэля_(Беляев)/1925_(СО)),
and the excerpt begins with Miss Adams. Don't silently substitute the later expanded novel.
The source edition is pre-1931, and the latest ordinary/wartime Russian term is expired.

**Odoevsky — clean literary text; correct the bibliographic simplification.** The
[source notes](https://ru.wikisource.org/wiki/4338-й_год_(Одоевский)) identify the more complete
1926 publication; `1835` is not sufficient as the first-publication date of all its material.
Even 1926 is beyond the US 95-year term as of 2026. Ushinsky's *Blind Horse* has no numeric
year in library.json. Its linked Wikisource page cites the modern site “Хранители сказок”
and supplies neither a dated print edition nor scans. **Needs edition/publication evidence**:
link and compare a historical edition rather than deriving a US term solely from the author's
death or assuming every word in the modern transcription has that provenance.

### Tatoeba's 242 sentence pairs — needs attribution, currently incomplete

Every sentence has `ru`, `en`, `audio`, `speaker`, `license`, `source`. **Zero have a text
`author` or text `credit` field**, and the shipped library has no English sentence IDs.
All 242 English IDs have now been recovered in credits_evidence.json by matching the import
cache to the exact current pair. Neither cache contained text contributor names. We have not
guessed them from the audio speaker. There are 484 text occurrences to attribute (some English
IDs repeat: there are 230 distinct English IDs), independently of the 242 audio credits.

[Current Tatoeba terms §§6.2, 6.5](https://tatoeba.org/en/terms_of_use) require author credit
and the text licence (default CC BY 2.0 FR); a translation can have its own author and rights.
An [archived older contributor agreement](https://en.wiki.tatoeba.org/articles/show/terms-of-use-v1)
accepted sentence links as an attribution mechanism. Links to both sides are useful now, but
we have not established which agreement covered each contribution, so **we do not declare
the links alone sufficient under the current terms**. Credits contains both IDs/URLs and an
honest incomplete attribution string for each pair. Recover names and modification history
from the contributor-bearing export/API/page history, and record both sides' actual licence.
No ShareAlike obligation comes merely from CC BY 2.0 FR. Audio's CC BY 4.0 or CC0 is not
the sentence's text licence. If a sentence was actually CC0, establish that per ID first.

### Original course, stories, mathematics — clean as project content, with honest authorship

`build_course.py` and its course modules contain project-written teaching examples and explanations;
ordinary vocabulary and grammatical facts aren't owned by a dictionary. Original drafts are
not imports under a third-party content licence. The project's supplied authorship account is
**Gemini 4 Argon / GPT (Codex) drafts edited by the project**, not solely human-authored prose.
`series.json` and `math.json` are **not present in this checkout**: there is no content to clear
here, and no item-level model logs from which to verify those optional collections' authorship.
Credits states both facts. It does not purport to grant a project-wide licence: no top-level
LICENSE/COPYING file was found in this checkout. Public visibility on GitHub is not itself
an open-source/content licence. Root should choose and scope its licence separately from imports.

### Morphology and paradigms — needs attribution and a documented derivation decision

`morphology_sources.md` contradicts a blanket “no dictionary sources” description: it names
English/Russian Wiktionary, Tikhonov (2002), Kuznetsova/Efremova (1986), Vasmer in Trubachyov's
Russian edition, Zaliznyak, Krysin, LSJ, and Lewis & Short. It says Wiktionary was the first
reference for every segmentation and Greek/Latin source. The short English learner notes in
`build_morphology.py` are project phrasing, but **their analysis is reference-derived**.
Don't label those dictionaries public domain en masse (especially modern translations/editions).
Facts, roots and standard forms differ from copied prose, a creative table arrangement or a
substantial protected selection. Keep original explanations and record which data was copied;
check any close paraphrase or selection against the actual reference before claiming independence.

`paradigm_sources.md` explicitly says the validator's tables were hand-copied from Wiktionary.
There are **51 literal reference keys**: 30 full nouns, two singular-only nouns, three adjectives,
10 verbs with imperatives, three other verbs, three pronouns. `build_paradigms.py` implements
rules and irregular overrides; its output isn't automatically a verbatim Wiktionary table.
Nonetheless, publicly shipping copied tests on GitHub counts even when tests aren't in the app.

The current [Wiktionary terms](https://en.wiktionary.org/wiki/Wiktionary:Copyrights) and
[Wikimedia terms §7](https://foundation.wikimedia.org/wiki/Policy:Terms_of_Use) use **CC BY-SA 4.0**,
not a universal 3.0. Historical revisions may be 3.0; the repository records neither revision
nor import date per table. Record the actual revision, its licence and contributor-history link.
The generated credits reconstruct all 51 English Wiktionary entry/history URLs from the
validator keys; these are attribution leads, explicitly not invented historical revision IDs.

**Recommended decision:** isolate and label the copied reference datasets and any genuinely
adapted expressive notes under the applicable BY-SA terms (4.0 for contemporary imports),
with attribution, source revision and modifications. For any retained 3.0 source, document its
permitted upgrade rather than pretending it originated under 4.0. Independently generated
grammatical facts and the Swift/Python algorithms can retain their own licence. A blanket
relicensing of all app code is unnecessary. Citation alone does not discharge SA; nor does a
CC label cure copied expression from a non-open dictionary. If the root chooses the facts-only
route, document the independent derivation and remove/rewrite protected expression, not useful facts.

## Strokes and fonts

| Item | Evidence / obligations | Verdict |
|---|---|---|
| strokes.json | build_strokes.py authors geometric line/arc/cubic paths in a strip aligned to SF Rounded metrics. No extracted system font binary/outlines are bundled by this generator. Common letter shapes/methods are distinct from copied artistic artwork. | **Clean** on inspected original-path implementation |
| strokes-cursive.json | build_strokes_cursive.py authors paths but cites Горецкий/Федосова's Прописи and English/Russian Wikipedia for shapes, order and instructional methodology. No workbook scan is bundled. Do not copy expressive workbook prose or page illustrations; independently describing a writing technique is different. Wikipedia images have individual file licences, not necessarily its text licence. | **Needs a decision** on whether any protected diagrams/prose were adapted; record edition/page/revision, otherwise describe as independently drawn paths based on school conventions |
| hershey-ink.json | Only DrusskyTests references it; it is a synthetic test fixture, not a production font. Exact Hershey distribution, generator and licence notice aren't recorded. | **Needs attribution/provenance repair** in the public test source |

The common Usenet [Hershey distribution notice](https://raw.githubusercontent.com/bjnortier/hershey/develop/HERSHEY-LICENSE)
permits use with acknowledgement of A. V. Hershey and the data format's creator James Hurt,
and disallows conversion to the specified NTIS distribution format. JSON ink isn't that format.
Do not replace a missing source chain with a blanket “public domain” statement: find the actual
package used, retain its notice and the generator/variant recipe. If it is that distribution,
include its required acknowledgements beside the fixture. This report does not assert that
an unrecorded package is identical to the linked one.

All **five** fonts listed by fonts.json exist, and each named licence file contains the complete
OFL 1.1 including definitions, conditions, termination and disclaimer. The generator embeds the
full text too, so a Sources view can display it without assuming folder preservation by Xcode.

| Font | Bundled notice / credited author | Reserved name |
|---|---|---|
| Caveat | Copyright 2014 The Caveat Project Authors | None declared in supplied notice |
| Bad Script | Copyright 2011 The Bad Script Project Authors | None declared in supplied notice |
| Neucha | Copyright 2008–2010 Jovanny Lemonad; binary additionally says 2005–2010 | None declared in supplied notice |
| Comforter Brush | Copyright 2015 The Comforter Brush Project Authors | None declared in supplied notice |
| Marck Script | Copyright 2011 Denis Masharov and Marck Fogel | **Marck Script** |

**Fonts — clean on inspected bundling/naming, preserve notices.** The file `Caveat-Variable.ttf`
is named locally but the binary retains the Caveat family; changing a filename isn't an RFN
violation. TTF family/PostScript names match fonts.json; Marck Script's reserved name and
copyright are retained. No font-editing/subsetting pipeline was found. This is not a binary
comparison against every original upstream release: if a font was modified elsewhere, that
history still needs checking. A modified Marck Script must use a non-reserved name absent
permission. Embed/distribute with its copyright and full OFL; don't sell fonts by themselves
or use author names for endorsement. OFL does not require the app or rendered notebook pages
to use OFL. [Official OFL](https://openfontlicense.org/open-font-license-official-text/).

## Models, algorithms and code

**ZIPA — needs attribution/NC-SA when installed; absent here.** `scripts/models.py` fetches
`anyspeech/zipa-small-crctc-ns-700k` at revision `797523ffb49e31e821a972313813cc4a26d902ef`.
The [model card](https://huggingface.co/anyspeech/zipa-small-crctc-ns-700k) says CC BY-NC-SA 4.0,
matching the script. Conversion to Core ML, fixed window and default w8 quantization are
recorded modifications; keep that model under NC-SA on distribution and credit Jian Zhu and
collaborators / AnySpeech, with the ZIPA paper and source. Do not apply a code repository's
licence to weights by analogy. Personal inference is supported; commercial redistribution/use
of the licensed weights requires a separate permission or replacement. The fetcher records
licence text only as a label in `ZipaPhones.json`; add the licence URL and preserved legal
notice to installed model assets. The conditional credit is ready in credits.json.

**FSRS-5 — clean as this project's implementation of published mathematics.**
`LearningCore/FSRS.swift` cites Ye et al./Open Spaced Repetition and implements the stated
equations and default weights. No dependency package is linked. Algorithms and numerical
parameters don't automatically import a software copyright licence. Courtesy credit is included.
If any substantial upstream code was translated/copied, preserve the full upstream
[MIT notice, copyright 2022 open-spaced-repetition](https://raw.githubusercontent.com/open-spaced-repetition/fsrs4anki/main/LICENSE).
Reading the Swift source did not establish such a copy; don't claim code originality was
proven by the absence of a package manager.

**Swift dependencies — clean within inspected scope.** `project.yml` has no packages and
only the local LearningCore target dependency. Swift imports are Apple frameworks or LearningCore;
no vendored third-party Swift package or Package.resolved was found. Apple SDK/system frameworks
are governed by Apple's development/platform agreements, not a bundled open-source credit.
This does not audit unrelated Rust/web dependencies elsewhere in rufren.

**Python/tooling — not shipped; clean for the identified local workflow.** The pinned Silero
requirements and installed `.dist-info` metadata were inspected. filelock/setuptools: MIT;
fsspec/MarkupSafe/networkx: BSD-3-Clause; Jinja2/mpmath/sympy: BSD-family notices;
typing_extensions: PSF-2.0. torch 2.15.0 declares a composite of Apache-2.0, Apache-2.0 with
LLVM exception, BSD-2/3-Clause, BSL-1.0, ISC, MIT and Zlib, with bundled third-party notices.
Don't simplify it to a single licence if redistributing the venv. ZIPA conversion pins torch,
onnx, coremltools and numpy; narration uses ctc-forced-aligner/MMS, and Whisper for checking.
These executables/models aren't app resources. Shipping their generated timing numbers doesn't
by itself distribute their code or weights. Review their separate package/model terms if the
toolchain itself is later published as a binary environment.

The new VoxCPM2/ESpeech narrator hooks in audio_store.py are research/tooling paths: none of
the audited three public manifests identifies those engines. This audit does not bless their
future outputs or all research artifacts; regenerate the inventory and verify their exact
weights, reference-voice provenance and terms before switching the shipped corpus.

## Live services and model-authored material

**Claude API — clean for normal tutoring/output use under the service contract.**
[Anthropic Commercial Terms §B](https://www.anthropic.com/legal/commercial-terms) assign its
rights, if any, in outputs to the customer to the extent the law allows; they do not impose a
CC licence or routine copyright-credit requirement on every tutor answer. That does not
clear third-party material reproduced in an answer or confer a copyright on entirely machine
generated expression. Terms and applicable usage policies still apply. Identify Claude honestly
as the tutor; don't describe it as a human teacher or a source licence for imported texts.

**Gemini/Codex drafting — authorship disclosure, not third-party copyright clearance.**
The one-paragraph model-authorship note in credits follows the project's supplied account,
including the exact name Gemini 4 Argon. No model identifier is inferred from contemporary
marketing names. No separate third-party content licence is evidenced for original drafts.
The particular generating account/product agreement has not been supplied; this audit does
not invent a universal commercial-output guarantee for those sessions. Provider output rights
are contractual and do not erase underlying source rights. Google's
[Gemini API terms](https://ai.google.dev/gemini-api/terms) could not be retrieved by the available
web reader; the exact generating surface and terms should be retained with future authoring
records. Official OpenAI documentation likewise points output ownership to the user's
agreement, rather than establishing the agreement used for these Codex drafts. The app doesn't
ship Gemini, GPT or Claude model weights.

## Sources-page contract and remaining work

Render the supplied `attribution` strings with their source/licence links and processing notes,
not just an aggregated author list. Provide font full texts. Keep PD/CC0 courtesy credits
separate from mandatory ones, foundation repository assets separate from the app, and ZIPA
conditional on installation. Never turn `credit_status: incomplete` into an anonymous but
apparently finished credit. Tatoeba entries are intentionally incomplete; 27 Commons entries
also need source checks. A successful `--check` is not a statement that those issues vanished.

The existing `CreditsView` in `Views/ContentsView.swift` only decodes `sources.json`, does not
decode its `author` at all, displays `credit` plus a licence label without a licence link or
change notice, omits pairs/narrations/texts/reference materials/models, and hard-codes Marck
Script while omitting its co-author Marck Fogel and the other four fonts. Thus having provenance
on disk has not ensured the attribution appears in the app.

Next integration: replace that reader with the generated Sources manifest, regenerate the Xcode project using xcodegen after
adding credits.json, and inspect the built resource bundle. This deputy writes audit files only;
it does not hand-edit the project or run an app build. Then resolve the finite attribution
queue, the Zamyatin edition, the copied-data licence boundary and the release exclusion of
Milena. Preserve original source-page metadata/licence snapshots at acquisition. Future public
audio distribution should have an explicit allowlist, rather than treating `kind: human_recording`
or “non-commercial” as sufficient permission.

## Integration status, 2026-11-08 (credits lane, after the audit)

What the audit asked for and what the app now does; `python3 apple/scripts/build_credits.py --check`
is the freshness gate, the Swift test `CreditsTests` the bundle gate (every bundled clip credited
exactly once; Tatoeba credits follow `library.json`).

- **Sources page.** `Views/CreditsView.swift` renders `credits.json` (entry «Исто́чники · Sources»
  in Settings): contributors with clips, licences as recorded, source links, processing notes;
  narrations with cuts; Tatoeba sentences with author and translator; Silero and conditional ZIPA;
  all five fonts in their own faces with the full OFL text; Wiktionary entries with history links;
  the model-authored-content statement; open items in red. The old `sources.json` reader is gone.
- **Tatoeba contributors.** `scripts/fetch_provenance.py tatoeba` fetched the owner and licence of all
  242 Russian sentences and their 230 English translations from the Tatoeba API (≤2 req/s, cached
  under `apple/.build/tatoeba/`) into `library.json` (`ru_author`, `en_author`, `ru_license`,
  `en_license`, `en_source`): 42 Russian authors, 37 translators, 0 missing. Three halves are
  CC0 1.0 rather than CC BY 2.0 FR; their composite licence strings say so.
- **Commons placeholders.** `fetch_provenance.py commons` read the file-page text of all 36 human
  clips with a missing or "not stated" author: 33 carry "Audio sample provided by The Shtooka
  Project" (credited as the other Shtooka takes are), люк is 1640max, ё is Pogrebnoj-Alexandroff,
  ю is Cherus. `author_evidence` on each entry records where the name came from. 0 remain.
- **Milena.** `e-bea15c.m4a` («е») was regenerated with Silero; no macOS speech is bundled, and
  `make_audio.py` no longer has a macOS take path. It now records author, licence and source page
  for every human recording and refuses to build one it cannot credit.
- **Editions.** Zamyatin cites the 1927 Prague first publication (US PD since 2023-01-01), the 1952
  first complete edition and the 2003 text Wikisource transcribes; all 26 excerpt sentences were
  checked verbatim against Wikisource's «Запись 1-ая». Odoevsky notes the 1835/1840 fragments and
  the 1926 complete text, with which carried the first letter unverified. Ushinsky «Слепая лошадь»
  is marked edition unverified. The Lensky narration keeps its PD claim as a claim
  (`narration_license` in `library.json`, `license_note` in `sources-narration.json`).
- **Project licence.** Top-level `LICENSE.md`: code all rights reserved until ember chooses a
  licence; third-party, NC-SA model and model-authored scope.

Still open (decisions, not fetches): the code licence; whether a public pack should carry Silero
(CC BY-NC-SA) output at all; the Wiktionary copied-expression boundary and revision pins (item 7
above); cursive/Hershey fixture provenance (item 8); dated scans for Ushinsky and the Odoevsky
first letter; US first-publication evidence for the 1894 Lensky recording.
