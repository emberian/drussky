//! A voice original linked locally to one exact saved web/Pencil drawing.
//! This is a connected writing activity, never an inferred practice Attempt.
use crate::{
    audio_studio::{AudioReviewPlayer, AudioSpeaking, StudioRecording, WritingRecorderScope},
    handwriting::{
        HandwritingActivity, HandwritingTask, WritingOriginalRef, WritingRoundReview, WritingStage,
    },
    model::AppState,
    repository::Result,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{cell::Cell, rc::Rc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WritingSpeechBinding {
    pub run_id: Uuid,
    pub round_id: Uuid,
    pub ink_original_id: Uuid,
    pub task_id: String,
    pub task_revision: String,
    pub ink_sha256: String,
    pub ink_format: String,
    pub model_audio_id: String,
    pub model_sha256: String,
}
impl WritingSpeechBinding {
    pub(crate) fn from_run(run: &HandwritingActivity) -> Result<Self> {
        if !matches!(
            run.stage,
            WritingStage::Compare | WritingStage::Say | WritingStage::Complete
        ) {
            return Err("Compare your saved drawing before opening its speaking turn.".into());
        }
        let original = run
            .original
            .as_ref()
            .ok_or("Keep the drawing before linking a voice to it.")?;
        run.definition()?;
        Self::from_original(run.id, run.round_id, &run.scope(), &run.task(), original)
    }
    fn from_original(
        run_id: Uuid,
        round_id: Uuid,
        scope: &str,
        task: &HandwritingTask,
        original: &WritingOriginalRef,
    ) -> Result<Self> {
        let authored = rufren_content::handwriting_tasks::handwriting_tasks()
            .into_iter()
            .find(|candidate| {
                candidate.revision == task.revision
                    && (task.id == format!("{}-copy", candidate.id)
                        || task.id == format!("{}-sound", candidate.id))
                    && task.reference_audio_id.as_deref() == Some(candidate.audio_id.as_str())
                    && task.target == candidate.target
            })
            .ok_or("This drawing’s exact authored task and spoken model are unavailable.")?;
        if run_id.is_nil()
            || round_id.is_nil()
            || original.id.is_nil()
            || original.scope_id != scope
            || original.task_id != task.id
            || original.task_revision != task.revision
        {
            return Err(
                "This drawing belongs to another writing task. Its original is kept.".into(),
            );
        }
        let bytes = crate::foundation_audio::bytes(&authored.audio_id)
            .ok_or("This task’s exact spoken model is unavailable.")?;
        Ok(Self {
            run_id,
            round_id,
            ink_original_id: original.id,
            task_id: authored.id,
            task_revision: authored.revision,
            ink_sha256: original.sha256.clone(),
            ink_format: original.format.clone(),
            model_audio_id: authored.audio_id,
            model_sha256: format!("{:x}", Sha256::digest(bytes)),
        })
    }

    fn scope(&self) -> WritingRecorderScope {
        WritingRecorderScope {
            run_id: self.run_id,
            round_id: self.round_id,
            ink_original_id: self.ink_original_id,
            ink_sha256: self.ink_sha256.clone(),
            ink_format: self.ink_format.clone(),
            task_id: self.task_id.clone(),
            task_revision: self.task_revision.clone(),
        }
    }
    fn key(&self) -> String {
        format!(
            "handwriting.speech:{}:{}:{}",
            self.run_id, self.round_id, self.ink_original_id
        )
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    version: u32,
    binding: WritingSpeechBinding,
    recording: Option<StudioRecording>,
}
fn immutable(record: &StudioRecording) -> StudioRecording {
    let mut value = record.clone();
    value.annotations.clear();
    value
}

impl AppState {
    fn check_writing_speech(
        &self,
        binding: &WritingSpeechBinding,
        editing: bool,
    ) -> Result<HandwritingActivity> {
        let run = self.active_handwriting()?;
        if WritingSpeechBinding::from_run(&run)? != *binding
            || (editing && !matches!(run.stage, WritingStage::Compare | WritingStage::Say))
        {
            return Err(
                "The speaking turn changed. Your voice and drawing originals are kept.".into(),
            );
        }
        let original = run
            .original
            .as_ref()
            .ok_or("The saved drawing is missing.")?;
        self.check_writing_speech_original(original)?;
        Ok(run)
    }
    fn check_writing_speech_original(&self, original: &WritingOriginalRef) -> Result<()> {
        // writing_artifact_ref verifies the persisted payload hash as well as identity.
        if self.writing_artifact_ref(original.id)? != *original {
            return Err("The drawing’s saved identity does not match this speaking turn.".into());
        }
        Ok(())
    }
    fn validate_writing_record(
        &self,
        binding: &WritingSpeechBinding,
        record: &StudioRecording,
        partial: bool,
    ) -> Result<()> {
        let metadata: rufren_core::AudioRecordMetadata =
            serde_json::from_value(serde_json::to_value(record).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        metadata.validate().map_err(|e| e.to_string())?;
        let scope: WritingRecorderScope =
            serde_json::from_value(record.device["writing_scope"].clone())
                .map_err(|_| "This voice does not identify its saved drawing.")?;
        if scope != binding.scope()
            || record.role != "attempt"
            || !record.passage_id.is_empty()
            || !record.revision_id.is_empty()
            || record.device["writing_scope"]["origin"] != "microphone_capture"
            || record.device.get("dialogue_scope").is_some()
            || record.device.get("sound_focus_scope").is_some()
            || (record.interrupted && !partial)
        {
            return Err("Choose a microphone take for this exact drawing. Select an interrupted take explicitly if it is useful.".into());
        }
        Ok(())
    }
    pub(crate) fn writing_speech_selected(
        &self,
        binding: &WritingSpeechBinding,
    ) -> Result<Option<StudioRecording>> {
        self.check_writing_speech(binding, false)?;
        self.writing_speech_stored_selection(binding)
    }
    /// Read an accepted round's exact voice without replacing or consulting the active round.
    pub(crate) fn writing_speech_for_review(
        &self,
        review: &WritingRoundReview,
    ) -> Result<Option<StudioRecording>> {
        let Some(original) = &review.original else {
            return Ok(None);
        };
        let binding = WritingSpeechBinding::from_original(
            review.run_id,
            review.round_id,
            &review.scope_id,
            &review.task,
            original,
        )?;
        self.check_writing_speech_original(original)?;
        self.writing_speech_stored_selection(&binding)
    }
    fn writing_speech_stored_selection(
        &self,
        binding: &WritingSpeechBinding,
    ) -> Result<Option<StudioRecording>> {
        let Some(selected) = self.repo.get::<Selection>(&binding.key())? else {
            return Ok(None);
        };
        if selected.version != 1 || selected.binding != *binding {
            return Err(
                "The selected voice belongs to another drawing. Its original is kept.".into(),
            );
        }
        if let Some(record) = &selected.recording {
            self.validate_writing_record(binding, record, true)?;
            let stored = self
                .repo
                .get::<StudioRecording>(&format!("handwriting.speech.asset:{}", record.asset_id))?
                .ok_or("The saved voice’s metadata is missing. Its audio original is kept.")?;
            if immutable(&stored) != immutable(record) {
                return Err(
                    "The voice’s immutable metadata changed. Its earlier pointer is kept.".into(),
                );
            }
        }
        Ok(selected.recording)
    }
    fn keep_writing_record(
        &self,
        binding: &WritingSpeechBinding,
        record: &StudioRecording,
    ) -> Result<()> {
        self.validate_writing_record(binding, record, true)?;
        let run = self.check_writing_speech(binding, true)?;
        let original = run.original.ok_or("The saved drawing is missing.")?;
        let artifact = self
            .repo
            .get::<serde_json::Value>(&original.local_key)?
            .ok_or("The exact drawing original is unavailable.")?;
        let bytes = serde_json::to_vec(&artifact["payload"]).map_err(|e| e.to_string())?;
        if format!("{:x}", Sha256::digest(&bytes)) != binding.ink_sha256 {
            return Err("The saved drawing failed its integrity check. The voice original is kept separately.".into());
        }
        let key = format!("handwriting.speech.asset:{}", record.asset_id);
        if let Some(old) = self.repo.get::<StudioRecording>(&key)? {
            if immutable(&old) != immutable(record) {
                return Err(
                    "A different voice uses this take identity. Both originals are kept.".into(),
                );
            }
        } else {
            self.repo.set(&key, record)?;
        }
        Ok(())
    }
    pub(crate) fn writing_speech_select(
        &self,
        binding: &WritingSpeechBinding,
        record: Option<StudioRecording>,
        explicit_partial: bool,
    ) -> Result<()> {
        self.check_writing_speech(binding, true)?;
        if let Some(record) = &record {
            self.validate_writing_record(binding, record, explicit_partial)?;
            self.keep_writing_record(binding, record)?;
        }
        self.repo.set(
            &binding.key(),
            &Selection {
                version: 1,
                binding: binding.clone(),
                recording: record,
            },
        )
    }
    fn writing_speech_captured(
        &self,
        binding: &WritingSpeechBinding,
        record: StudioRecording,
    ) -> Result<()> {
        self.keep_writing_record(binding, &record)?;
        self.writing_speech_select(binding, (!record.interrupted).then_some(record), false)
    }
}

async fn play_model(asset: &str, owner: &str) -> Result<bool> {
    let bytes = crate::foundation_audio::bytes(asset).ok_or("This spoken model is unavailable.")?;
    let source =
        serde_json::to_string(&format!("data:audio/wav;base64,{}", STANDARD.encode(bytes)))
            .map_err(|e| e.to_string())?;
    let owner = serde_json::to_string(owner).map_err(|e| e.to_string())?;
    let script = format!(
        "const source={source},owner={owner};\nconst playing=(async()=>{{\n{}\n}})();\nconst model=window.rufrenFoundationAudio;window.rufrenWritingModels??=new Map();window.rufrenWritingModels.set(owner,model);try{{return await playing;}}finally{{if(window.rufrenWritingModels.get(owner)===model)window.rufrenWritingModels.delete(owner);}}",
        include_str!("foundation_audio.js")
    );
    document::eval(&script)
        .await
        .map(|value| value == serde_json::Value::Bool(true))
        .map_err(|e| e.to_string())
}
fn stop_model(owner: &str) {
    let owner = serde_json::to_string(owner).unwrap_or_default();
    let _ = document::eval(&format!(
        "window.rufrenWritingModels?.get({owner})?.cancel();"
    ));
}

const WRITING_SPEECH_STYLE: &str = r#"
.writing-speech .audio-studio{gap:8px;padding-bottom:0;margin:12px 0}
.writing-voice-controls{display:flex;flex-wrap:wrap;align-items:center;gap:8px}
.writing-voice-controls button{font-size:13px;padding:10px 13px}
.writing-speech #audio-status,.writing-speech #writing-voice-recovery p{margin:0;font-size:13px;line-height:1.5}
.writing-speech .audio-mic[data-active=false]{display:none}
.writing-speech #writing-voice-history{font-size:13px}
.writing-speech #writing-voice-history summary{cursor:pointer;padding:8px 0;color:var(--green)}
#writing-voice-takes{display:grid;gap:6px;margin:8px 0}
#writing-voice-takes button{text-align:left;font-size:13px;min-height:44px}
#writing-voice-takes button[aria-pressed=true]{outline:2px solid var(--green);outline-offset:-2px}
.writing-speech [hidden]{display:none!important}
"#;

#[component]
pub(crate) fn WritingSpeech(
    app: Signal<AppState>,
    binding: WritingSpeechBinding,
    #[props(default)] on_busy: EventHandler<bool>,
) -> Element {
    let identity = serde_json::to_string(&binding).unwrap_or_default();
    rsx! {for identity in [identity]{WritingSpeechEditor {key:"{identity}",app,binding:binding.clone(),on_busy}}}
}

#[component]
fn WritingSpeechEditor(
    mut app: Signal<AppState>,
    binding: WritingSpeechBinding,
    on_busy: EventHandler<bool>,
) -> Element {
    let owner = use_hook(|| Uuid::new_v4().to_string());
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let mut playing = use_signal(|| false);
    let mut recording = use_signal(|| false);
    let mut reported = use_signal(|| false);
    let mut problem = use_signal(String::new);
    use_effect(move || {
        let busy = playing() || recording();
        if reported() != busy {
            reported.set(busy);
            on_busy.call(busy);
        }
    });
    let drop_owner = owner.clone();
    let drop_alive = alive.clone();
    use_drop(move || {
        drop_alive.set(false);
        stop_model(&drop_owner);
        on_busy.call(false);
    });
    let loaded = app.read().writing_speech_selected(&binding);
    let selected = match loaded {
        Ok(value) => value,
        Err(error) => return rsx! {p {role:"alert","{error}"}},
    };
    let run = match app.read().check_writing_speech(&binding, false) {
        Ok(run) => run,
        Err(error) => return rsx! {p {role:"alert","{error}"}},
    };
    let readonly = run.stage == WritingStage::Complete;
    let model_binding = binding.clone();
    let capture_binding = binding.clone();
    let select_binding = binding.clone();
    let play_alive = alive.clone();
    let play_owner = owner.clone();
    rsx! {style {"{WRITING_SPEECH_STYLE}"}
        section {class:"writing-speech","data-writing-speech-run":binding.run_id.to_string(),"data-writing-speech-round":binding.round_id.to_string(),"data-writing-speech-original":binding.ink_original_id.to_string(),
            div {class:"actions",button {class:"primary",disabled:playing()||recording(),onclick:move |_|{
                playing.set(true);problem.set(String::new());let binding=model_binding.clone();let alive=play_alive.clone();let owner=play_owner.clone();
                spawn(async move{let result=play_model(&binding.model_audio_id,&owner).await;if !alive.get(){return;}playing.set(false);match result{Ok(true)=>{if let Err(error)=app.write().writing_audio_completed(binding.run_id,binding.round_id,&binding.model_audio_id){problem.set(error);}},Ok(false)=>problem.set("Playback stopped or interrupted. Replay when you are ready.".into()),Err(error)=>problem.set(error)}});
            },if playing(){"Playing…"}else{"Listen"}}
            if playing(){button {class:"text-button",onclick:move |_|stop_model(&owner),"Stop model"}}}
            if readonly {if let Some(record)=&selected{AudioReviewPlayer {asset_id:record.asset_id.clone(),expected_hash:Some(record.content_hash.clone()),label:"Your saved voice".to_string()}}}
            else{div {inert:playing().then_some(""),AudioSpeaking {passage_id:String::new(),revision_id:String::new(),writing_scope:Some(binding.scope()),selected_asset_id:selected.as_ref().map(|r|r.asset_id.clone()),selected_content_hash:selected.as_ref().map(|r|r.content_hash.clone()),workspace_id:if cfg!(target_arch="wasm32"){String::new()}else{app.read().device.to_string()},
                on_recording_state:move|busy|recording.set(busy),
                on_saved:move|record:StudioRecording|{let partial=record.interrupted;let result=app.write().writing_speech_captured(&capture_binding,record);match result{Ok(())=>problem.set(if partial{"Your partial take is kept. Open Earlier takes to choose it.".into()}else{String::new()}),Err(error)=>problem.set(error)}},
                on_selected:move|record:Option<StudioRecording>|{let result=app.write().writing_speech_select(&select_binding,record,true);match result{Ok(())=>problem.set(String::new()),Err(error)=>problem.set(error)}}
            }}}
            if selected.is_some(){p {class:"quiet","Your voice is saved with this drawing."}}
            if !problem().is_empty(){p {role:"status","{problem}"}}
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod writing_speech_tests {
    use super::*;
    use crate::{handwriting::SayDisposition, repository::Repository};
    use rufren_content::handwriting_tasks::{FoundationHandwritingSequence, handwriting_tasks};

    fn fixture() -> (AppState, HandwritingActivity, WritingSpeechBinding) {
        let app =
            AppState::from_repository(Rc::new(Repository::open_at(":memory:").unwrap())).unwrap();
        let task = handwriting_tasks()
            .into_iter()
            .find(|task| task.audio_id == "foundation-a")
            .unwrap();
        let id = Uuid::new_v4();
        let round_id = Uuid::new_v4();
        let mut run = HandwritingActivity {
            version: 2,
            id,
            round_id,
            attempt_id: Uuid::new_v4(),
            course_session: Uuid::new_v4(),
            course_step: task.foundation_step_id.clone(),
            course_step_revision: task.foundation_step_revision.clone(),
            task_id: task.id,
            task_revision: task.revision,
            sequence: FoundationHandwritingSequence::SeeCopyHearSay,
            stage: WritingStage::Say,
            scope_id: format!("{id}:{round_id}"),
            original: None,
            audio_completed: false,
            model_revealed: true,
            entry_previewed: true,
            say: SayDisposition::NotReported,
            history: vec![],
            native_instance: None,
        };
        run.original = Some(
            app.writing_store_artifact(
                Uuid::new_v4(),
                &run.scope(),
                &run.task().id,
                &run.task_revision,
                "web_ink",
                serde_json::json!({"pressure":[0.2,0.7],"points":[[1,2],[3,4]]}),
            )
            .unwrap(),
        );
        app.repo.set("handwriting.active", &id).unwrap();
        app.repo
            .set(&format!("handwriting.activity:{id}"), &run)
            .unwrap();
        let binding = WritingSpeechBinding::from_run(&run).unwrap();
        (app, run, binding)
    }
    fn take(binding: &WritingSpeechBinding) -> StudioRecording {
        let mut scope = serde_json::to_value(binding.scope()).unwrap();
        scope["origin"] = "microphone_capture".into();
        StudioRecording {
            asset_id: Uuid::new_v4().to_string(),
            content_hash: "a".repeat(64),
            role: "attempt".into(),
            name: "My voice".into(),
            duration_ms: 1000,
            sample_rate: 24000,
            channels: 1,
            frames: 24000,
            byte_length: 48044,
            mime_type: "audio/wav".into(),
            device: serde_json::json!({"writing_scope":scope}),
            ..Default::default()
        }
    }
    #[test]
    fn exact_drawing_take_survives_reload_without_attempt_or_passage() {
        let (app, _, binding) = fixture();
        let voice = take(&binding);
        app.writing_speech_captured(&binding, voice.clone())
            .unwrap();
        assert_eq!(
            app.writing_speech_selected(&binding).unwrap(),
            Some(voice.clone())
        );
        let reloaded = AppState::from_repository(app.repo.clone()).unwrap();
        assert_eq!(
            reloaded.writing_speech_selected(&binding).unwrap(),
            Some(voice)
        );
        assert_eq!(app.library.events.len(), reloaded.library.events.len());
        assert_eq!(app.library.attempts().count(), 0);
    }
    #[test]
    fn archived_voice_review_survives_another_active_task_without_adopting_competing_ink() {
        let (app, mut run, binding) = fixture();
        let voice = take(&binding);
        app.writing_speech_captured(&binding, voice.clone())
            .unwrap();
        run.stage = WritingStage::Complete;
        app.repo
            .set(&format!("handwriting.activity:{}", run.id), &run)
            .unwrap();
        app.writing_new_round(run.id, run.round_id, run.sequence)
            .unwrap();
        let other_task = handwriting_tasks()
            .into_iter()
            .find(|task| task.id != run.task_id)
            .unwrap();
        let mut other = run.clone();
        other.id = Uuid::new_v4();
        other.round_id = Uuid::new_v4();
        other.scope_id = format!("{}:{}", other.id, other.round_id);
        other.task_id = other_task.id;
        other.task_revision = other_task.revision;
        other.course_step = other_task.foundation_step_id;
        other.course_step_revision = other_task.foundation_step_revision;
        other.stage = WritingStage::Writing;
        other.original = None;
        app.repo
            .set(&format!("handwriting.activity:{}", other.id), &other)
            .unwrap();
        app.repo.set("handwriting.active", &other.id).unwrap();

        let review = app
            .handwriting_round_review(run.id, binding.round_id)
            .unwrap();
        assert_eq!(
            app.writing_speech_for_review(&review).unwrap(),
            Some(voice.clone())
        );
        assert_eq!(app.active_handwriting().unwrap(), other);
        assert!(app.writing_speech_selected(&binding).is_err());
        let competing = app
            .writing_store_artifact(
                Uuid::new_v4(),
                &review.scope_id,
                &review.task.id,
                &review.task.revision,
                "web_ink",
                serde_json::json!({"pressure":[0.3],"points":[[5,6]]}),
            )
            .unwrap();
        let mut recovered = app
            .handwriting_round_review(run.id, binding.round_id)
            .unwrap();
        assert!(
            recovered
                .originals
                .iter()
                .any(|original| original.reference == competing)
        );
        recovered.original = None;
        assert!(app.writing_speech_for_review(&recovered).unwrap().is_none());
        recovered.original = Some(competing);
        assert!(app.writing_speech_for_review(&recovered).unwrap().is_none());

        let metadata_key = format!("handwriting.speech.asset:{}", voice.asset_id);
        let mut changed = voice.clone();
        changed.content_hash = "c".repeat(64);
        app.repo.set(&metadata_key, &changed).unwrap();
        assert!(app.writing_speech_for_review(&review).is_err());
        app.repo.set(&metadata_key, &voice).unwrap();
        let original = review.original.as_ref().unwrap();
        let mut artifact = app
            .repo
            .get::<serde_json::Value>(&original.local_key)
            .unwrap()
            .unwrap();
        artifact["payload"]["pressure"] = serde_json::json!([0.9]);
        app.repo.set(&original.local_key, &artifact).unwrap();
        assert!(app.writing_speech_for_review(&review).is_err());
        assert_eq!(app.active_handwriting().unwrap(), other);
    }
    #[test]
    fn changed_round_artifact_origin_and_bytes_cannot_replace_pointer() {
        let (app, mut run, binding) = fixture();
        let voice = take(&binding);
        app.writing_speech_captured(&binding, voice.clone())
            .unwrap();
        for field in ["round_id", "ink_original_id", "ink_sha256"] {
            let mut wrong = voice.clone();
            wrong.device["writing_scope"][field] = if field == "ink_sha256" {
                "b".repeat(64).into()
            } else {
                Uuid::new_v4().to_string().into()
            };
            assert!(
                app.writing_speech_select(&binding, Some(wrong), true)
                    .is_err()
            );
        }
        let mut imported = voice.clone();
        imported.device["writing_scope"]["origin"] = "imported_audio".into();
        assert!(
            app.writing_speech_select(&binding, Some(imported), true)
                .is_err()
        );
        let mut changed = voice.clone();
        changed.content_hash = "c".repeat(64);
        assert!(
            app.writing_speech_select(&binding, Some(changed), true)
                .is_err()
        );
        assert_eq!(
            app.writing_speech_selected(&binding).unwrap(),
            Some(voice.clone())
        );
        let original = run.original.as_ref().unwrap();
        let mut artifact = app
            .repo
            .get::<serde_json::Value>(&original.local_key)
            .unwrap()
            .unwrap();
        artifact["payload"]["pressure"] = serde_json::json!([0.9]);
        app.repo.set(&original.local_key, &artifact).unwrap();
        assert!(
            app.writing_speech_select(&binding, Some(voice), true)
                .is_err()
        );
        run.round_id = Uuid::new_v4();
        app.repo
            .set(&format!("handwriting.activity:{}", run.id), &run)
            .unwrap();
        assert!(app.writing_speech_select(&binding, None, true).is_err());
    }
    #[test]
    fn interrupted_original_requires_explicit_adoption_and_saved_turn_is_readonly() {
        let (app, mut run, binding) = fixture();
        let mut voice = take(&binding);
        voice.interrupted = true;
        app.writing_speech_captured(&binding, voice.clone())
            .unwrap();
        assert!(app.writing_speech_selected(&binding).unwrap().is_none());
        assert!(
            app.writing_speech_select(&binding, Some(voice.clone()), false)
                .is_err()
        );
        app.writing_speech_select(&binding, Some(voice.clone()), true)
            .unwrap();
        run.stage = WritingStage::Complete;
        app.repo
            .set(&format!("handwriting.activity:{}", run.id), &run)
            .unwrap();
        assert_eq!(app.writing_speech_selected(&binding).unwrap(), Some(voice));
        assert!(app.writing_speech_select(&binding, None, true).is_err());
    }
}
