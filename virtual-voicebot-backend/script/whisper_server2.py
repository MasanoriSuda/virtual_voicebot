import asyncio
import json
import logging
import struct
import wave

from fastapi import FastAPI, UploadFile, File, HTTPException, WebSocket, WebSocketDisconnect
import uvicorn
import tempfile
import os
import torch

app = FastAPI()
logger = logging.getLogger(__name__)

MAX_UPLOAD_SIZE = int(os.environ.get("ASR_MAX_UPLOAD_SIZE_BYTES", str(25 * 1024 * 1024)))
UPLOAD_READ_CHUNK_SIZE = 1024 * 1024
ASR_INFERENCE_CONCURRENCY = int(os.environ.get("ASR_INFERENCE_CONCURRENCY", "1"))
ASR_INFERENCE_TIMEOUT_SECONDS = float(os.environ.get("ASR_INFERENCE_TIMEOUT_SECONDS", "120"))
ASR_WS_IDLE_TIMEOUT_SECONDS = float(os.environ.get("ASR_WS_IDLE_TIMEOUT_SECONDS", "30"))
ASR_INFERENCE_SEMAPHORE = asyncio.Semaphore(max(1, ASR_INFERENCE_CONCURRENCY))

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
        logger.exception("failed to initialize KANA_CONVERTER; falling back to raw text output")
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
        logger.exception("failed to apply KANA_CONVERTER; returning original text")
        return text


def _ulaw_to_linear16_sample(code: int) -> int:
    ulaw = (~code) & 0xFF
    sign = ulaw & 0x80
    exponent = (ulaw >> 4) & 0x07
    mantissa = ulaw & 0x0F
    magnitude = ((mantissa << 3) + 0x84) << exponent
    pcm = magnitude - 0x84
    return -pcm if sign else pcm


ULAW_TO_PCM16_TABLE = tuple(
    struct.pack("<h", _ulaw_to_linear16_sample(code)) for code in range(256)
)


def ulaw_to_linear16_bytes(mulaw_bytes: bytes) -> bytes:
    return b"".join(ULAW_TO_PCM16_TABLE[b] for b in mulaw_bytes)


async def run_asr_inference(tmp_path: str) -> str:
    # グローバルモデルへの同時アクセスを抑制し、推論ハングを timeout で遮断する。
    try:
        async with ASR_INFERENCE_SEMAPHORE:
            raw_text = await asyncio.wait_for(
                asyncio.to_thread(transcribe_audio, tmp_path),
                timeout=ASR_INFERENCE_TIMEOUT_SECONDS,
            )
    except asyncio.TimeoutError as e:
        raise HTTPException(status_code=504, detail="ASR inference timeout") from e
    return apply_output_script(raw_text)


def write_mulaw_to_wav(mulaw_bytes: bytes, path: str) -> None:
    pcm16 = ulaw_to_linear16_bytes(mulaw_bytes)
    with wave.open(path, "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(8000)
        wf.writeframes(pcm16)


@app.get("/healthz")
async def healthz():
    return {"status": "ok", "engine": ASR_ENGINE}

@app.post("/transcribe")
async def transcribe(file: UploadFile = File(...)):
    tmp_path = None
    try:
        content_type = (file.content_type or "").lower()
        if not (content_type.startswith("audio/") or content_type == "application/octet-stream"):
            raise HTTPException(
                status_code=400,
                detail=f"unsupported content_type: {file.content_type}",
            )

        # 一時ファイルに保存
        with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as tmp:
            tmp_path = tmp.name
            total_size = 0
            while True:
                chunk = await file.read(UPLOAD_READ_CHUNK_SIZE)
                if not chunk:
                    break
                total_size += len(chunk)
                if total_size > MAX_UPLOAD_SIZE:
                    raise HTTPException(
                        status_code=413,
                        detail=f"upload too large: max={MAX_UPLOAD_SIZE} bytes",
                    )
                tmp.write(chunk)

        if total_size == 0:
            raise HTTPException(status_code=400, detail="empty upload")

        text = await run_asr_inference(tmp_path)
        return {"text": text}
    finally:
        await file.close()
        if tmp_path is not None:
            try:
                os.unlink(tmp_path)
            except FileNotFoundError:
                pass


@app.websocket("/transcribe_stream")
async def transcribe_stream(websocket: WebSocket):
    await websocket.accept()
    total_size = 0
    sent_first_partial = False
    pcm_mulaw = bytearray()

    try:
        while True:
            try:
                message = await asyncio.wait_for(
                    websocket.receive(), timeout=ASR_WS_IDLE_TIMEOUT_SECONDS
                )
            except asyncio.TimeoutError:
                logger.warning(
                    "transcribe_stream idle timeout: timeout_sec=%s bytes=%s",
                    ASR_WS_IDLE_TIMEOUT_SECONDS,
                    total_size,
                )
                await websocket.close(code=1001)
                return
            msg_type = message.get("type")

            if msg_type == "websocket.disconnect":
                raise WebSocketDisconnect()

            if msg_type != "websocket.receive":
                continue

            data = message.get("bytes")
            if data is not None:
                total_size += len(data)
                if total_size > MAX_UPLOAD_SIZE:
                    await websocket.send_json(
                        {
                            "type": "error",
                            "error": f"upload too large: max={MAX_UPLOAD_SIZE} bytes",
                        }
                    )
                    await websocket.close(code=1009)
                    return

                pcm_mulaw.extend(data)
                if not sent_first_partial:
                    # first-partial timeout を満たすための heartbeat（実テキストは final で返す）
                    await websocket.send_json({"type": "partial", "text": ""})
                    sent_first_partial = True
                continue

            text = message.get("text")
            if text is None:
                continue

            try:
                payload = json.loads(text)
            except json.JSONDecodeError:
                await websocket.send_json({"type": "error", "error": "invalid JSON control message"})
                await websocket.close(code=1003)
                return

            if payload.get("type") != "end":
                continue

            if not sent_first_partial:
                await websocket.send_json({"type": "partial", "text": ""})
                sent_first_partial = True

            if not pcm_mulaw:
                await websocket.send_json({"type": "final", "text": ""})
                await websocket.close()
                return

            tmp_path = None
            try:
                with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as tmp:
                    tmp_path = tmp.name
                write_mulaw_to_wav(bytes(pcm_mulaw), tmp_path)
                text = await run_asr_inference(tmp_path)
                await websocket.send_json({"type": "final", "text": text})
            except HTTPException as e:
                await websocket.send_json({"type": "error", "error": str(e.detail)})
            except Exception:
                logger.exception("transcribe_stream internal error")
                await websocket.send_json(
                    {"type": "error", "error": "internal server error"}
                )
            finally:
                if tmp_path is not None:
                    try:
                        os.unlink(tmp_path)
                    except FileNotFoundError:
                        pass
            await websocket.close()
            return
    except WebSocketDisconnect:
        return

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=9010)
