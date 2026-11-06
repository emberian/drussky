"""Pinned public Qwen3 VoiceDesign assets and task-owned paths; no inference here."""
from pathlib import Path

PROJECT = Path(__file__).resolve().parents[2]
STATE = PROJECT / ".local" / "qwen3-tts"
MODEL_ID = "mlx-community/Qwen3-TTS-12Hz-1.7B-VoiceDesign-bf16"
MODEL_REVISION = "7d3824abff87e49756bb0f83fb5411de75d160c4"
UPSTREAM_ID = "Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign"
UPSTREAM_REVISION = "5ecdb67327fd37bb2e042aab12ff7391903235d3"
RUNTIME_VERSION = "0.6.4"
MODEL_PATH = STATE / "models" / MODEL_REVISION
MANIFEST = STATE / "model-manifest.json"
TOKEN_FILE = STATE / "worker.token"
PID_FILE = STATE / "worker.pid"
LOG_FILE = STATE / "logs" / "worker.log"
VOICE_ID = "rufren-russian-warm-v1"
PROMPT_VERSION = "rufren-qwen-russian-v2"
WORKER_SCHEMA = "rufren-qwen-worker-v2"
VOICE_DESCRIPTION = (
    "A warm adult female native Russian speaker using natural standard Russian, "
    "with clear articulation and a measured conversational pace. Accurate Russian "
    "word stress, natural sentence intonation, no theatrical delivery."
)
MAX_TEXT_BYTES = 1024
MAX_TEXT_CHARS = 256
MAX_PCM_BYTES = 24000 * 2 * 45
MAX_REQUEST_BYTES = 8192
MAX_RESPONSE_BYTES = 4 * 1024 * 1024
