from fastapi import FastAPI, UploadFile, File
import uvicorn
import tempfile
import os
import torch

app = FastAPI()

ASR_OUTPUT_SCRIPT = os.environ.get("ASR_OUTPUT_SCRIPT", "hiragana").lower()
KANA_CONVERTER = None
if ASR_OUTPUT_SCRIPT in ("hiragana", "katakana"):
    try:
        from pykakasi import kakasi

        _k = kakasi()
        if ASR_OUTPUT_SCRIPT == "hiragana":
            _k.setMode("J", "H")
            _k.setMode("K", "H")
            _k.setMode("H", "H")
        else:
            _k.setMode("J", "K")
            _k.setMode("K", "K")
            _k.setMode("H", "K")
        KANA_CONVERTER = _k.getConverter()
    except Exception:
        KANA_CONVERTER = None

ASR_ENGINE = os.environ.get("ASR_ENGINE", "kotoba").lower()

if ASR_ENGINE == "reazon":
    import nemo.collections.asr as nemo_asr

    model = nemo_asr.models.ASRModel.from_pretrained(
        "reazon-research/reazonspeech-nemo-v2"
    )

    def transcribe_audio(tmp_path: str) -> str:
        result = model.transcribe([tmp_path])
        return normalize_text(result)
else:
    from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor, pipeline

    CACHE_DIR = os.environ.get("HF_HOME", "/var/cache/huggingface")
    MODEL_ID = "kotoba-tech/kotoba-whisper-v2.2"

    device = "cuda:0" if torch.cuda.is_available() else "cpu"
    torch_dtype = torch.float16 if torch.cuda.is_available() else torch.float32
    attn_implementation = "sdpa"
    if torch.cuda.is_available():
        try:
            import flash_attn  # noqa: F401
            attn_implementation = "flash_attention_2"
        except Exception:
            attn_implementation = "sdpa"

    model = AutoModelForSpeechSeq2Seq.from_pretrained(
        MODEL_ID,
        torch_dtype=torch_dtype,
        low_cpu_mem_usage=True,
        attn_implementation=attn_implementation,
        cache_dir=CACHE_DIR,
    )
    model.to(device)
    processor = AutoProcessor.from_pretrained(MODEL_ID, cache_dir=CACHE_DIR)

    pipe_kwargs = {
        "model": model,
        "tokenizer": processor.tokenizer,
        "feature_extractor": processor.feature_extractor,
        "device": device,
    }
    try:
        pipe = pipeline(
            "automatic-speech-recognition",
            torch_dtype=torch_dtype,
            **pipe_kwargs,
        )
    except TypeError:
        pipe = pipeline(
            "automatic-speech-recognition",
            dtype=torch_dtype,
            **pipe_kwargs,
        )

    def transcribe_audio(tmp_path: str) -> str:
        result = pipe(tmp_path, generate_kwargs={"language": "ja", "task": "transcribe"})
        return result.get("text", "")


def normalize_text(result) -> str:
    if result is None:
        return ""
    if isinstance(result, str):
        return result
    if isinstance(result, dict):
        return normalize_text(result.get("text"))
    if isinstance(result, (list, tuple)):
        if not result:
            return ""
        return normalize_text(result[0])
    text = getattr(result, "text", None)
    if isinstance(text, str):
        return text
    return ""


def apply_output_script(text: str) -> str:
    if not text:
        return text
    if KANA_CONVERTER is None:
        return text
    try:
        return KANA_CONVERTER.do(text)
    except Exception:
        return text

@app.post("/transcribe")
async def transcribe(file: UploadFile = File(...)):
    # 一時ファイルに保存
    with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as tmp:
        data = await file.read()
        tmp.write(data)
        tmp_path = tmp.name

    # ASR 文字起こし
    text = apply_output_script(transcribe_audio(tmp_path))

    return {"text": text}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=9000)
