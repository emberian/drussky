"""Pinned personal-use Silero Russian comparator; no inference on import."""
from pathlib import Path
import sys
sys.dont_write_bytecode = True

PROJECT = Path(__file__).resolve().parents[2]
STATE = PROJECT / ".local" / "silero-tts"
MODEL_ID = "v5_6_ru"
MODEL_URL = "https://models.silero.ai/models/tts/ru/v5_6_ru.pt"
MODEL_BYTES = 147_664_410
# Filled from the verified first acquisition; setup then enforces this identity.
MODEL_SHA256 = "04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e"
SOURCE_REPO = "snakers4/silero-models"
SOURCE_REVISION = "e382298387332f2bf461fe6eadfde99104611b0c"
LICENSE = "CC-BY-NC-SA-4.0"
TORCH_VERSION = "2.15.0"
MODEL = STATE / "models" / f"{MODEL_ID}.pt"
MANIFEST = STATE / "model-manifest.json"
VOICE = "xenia"
RATE = 24_000
MAX_TEXT_BYTES = 1024
MAX_TEXT_CHARS = 256
MAX_PCM_BYTES = RATE * 2 * 45
MAX_REQUEST_BYTES = 8192
MAX_RESPONSE_BYTES = 4 * 1024 * 1024
PUBLIC_CASES = (
    {"request_id": "silero-stress-castle-v1", "text": "Это за́мок.", "meaning": "This is a castle."},
    {"request_id": "silero-stress-lock-v1", "text": "Это замо́к.", "meaning": "This is a lock."},
)
