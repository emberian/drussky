// Synthetic v3 persistence facts shared by shell-update and relay-recovery QA.
// These are labeled fixture observations; no teacher, recognizer or learner speech runs.
import {randomUUID} from "node:crypto";

export function addSyntheticSpeechFacts(backup, revision, writingAttempt) {
  const events = backup.library.events, device = writingAttempt.device;
  const add = (kind, data) => {
    const event = { version: 3, id: randomUUID(), device, recorded_at_ms: Date.now(), fact: { kind, data } };
    events[event.id] = event; return event.id;
  };
  const attempt = add("attempt_recorded", { session: randomUUID(), revision, skill: "speaking", response: "Synthetic QA recording label; no learner speech was recorded", cues: [], self_rating: "not_reported", policy: "shell-update-synthetic-fixture" });
  const asset = `shell-update-fixture-${randomUUID()}`, audioHash = "a".repeat(64);
  const recording = add("recording_attached", { asset_id: asset, asset: { sha256: audioHash, media_type: "audio/wav", sample_rate_hz: 16000, channels: 1, frames: 16000 }, target: { target: "attempt", id: attempt }, role: "attempt", note: "Synthetic metadata-only shell update fixture; no original learner audio exists." });
  const transcript = "Синтетическая запись для проверки обновления: ёжик.";
  const provenance = { schema_version: "rufren-speech-transcript-v1", request_id: randomUUID(), provider: "qa-shell-update-fixture", model: "synthetic-no-recognizer", model_revision: "fixture-v1", runtime: "fixture-only", manifest_sha256: "d".repeat(64), worker_sha256: "e".repeat(64) };
  const audit = { schema_version: provenance.schema_version, request_id: provenance.request_id, attempt_id: attempt, revision_id: revision, asset_id: asset, audio_sha256: audioHash, original_audio: { sample_rate: 16000, channels: 1, frames: 16000, encoding: "ieee_f32le", bits: 32 }, source_pcm_sha256: "b".repeat(64), source_pcm_hash_format: "interleaved-little-endian-f32-no-header-v1", span: { start_frame: 0, end_frame: 16000 }, span_coordinate_system: "original_frames_half_open", derivative: { sample_rate: 16000, channels: 1, frames: 16000, pcm_sha256: "c".repeat(64), pcm_hash_format: "little-endian-f32-no-header-v1", method: "synthetic persistence fixture; no DSP executed", scipy_version: "fixture", amplitude_normalization: false }, evidence_kind: "transcript_hypothesis", transcript, language: "ru", language_selection: "explicit_not_detected", target_conditioned: false, word_timestamps: null, word_alignment_status: "not_run", confidence: null, acoustic_assessment: null, provenance: { provider: provenance.provider, model: provenance.model, revision: provenance.model_revision, runtime: provenance.runtime, mlx_version: "fixture", manifest_sha256: provenance.manifest_sha256, worker_sha256: provenance.worker_sha256, decoding: {} }, timing: {}, usage: {} };
  const transcription = add("speech_transcribed", { recording, attempt, revision, asset_id: asset, audio_sha256: audioHash, span: audit.span, original_audio: { sample_rate_hz: 16000, channels: 1, frames: 16000, encoding: "ieee_f32le", bits: 32 }, transcript, language: "ru", provenance, audit_json: JSON.stringify(audit) });
  const confirmation = add("transcript_confirmed", { transcription, text: transcript, operation: "confirm", supersedes: [] });
  add("ai_assessment_recorded", { request_id: randomUUID(), attempt, revision, skill: "speaking", task: "pronunciation", response: transcript, context: null, confirmed_transcript: confirmation, evidence: "transcript", dimension: "grammar", verdict: "uncertain", suggested_response: null, corrections: [], explanation: "SYNTHETIC SHELL UPDATE ASSESSMENT: storage fixture only, not a language or pronunciation judgment.", next_step: "No learner action; verify durable storage only.", provenance: { provider: "qa-shell-update-fixture", model: "synthetic-no-teacher", response_id: randomUUID(), prompt_version: "persistence-fixture-v1" }, supersedes: [] });
  return { attempt, transcription, confirmation };
}
