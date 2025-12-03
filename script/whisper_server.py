from fastapi import FastAPI, UploadFile, File
import uvicorn
import tempfile
import whisper

app = FastAPI()
model = whisper.load_model("large-v3-turbo")  # 好きなモデルに

@app.post("/transcribe")
async def transcribe(file: UploadFile = File(...)):
    # 一時ファイルに保存
    with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as tmp:
        data = await file.read()
        tmp.write(data)
        tmp_path = tmp.name

    # Whisperで文字起こし
    result = model.transcribe(tmp_path, language="ja")
    text = result.get("text", "")

    return {"text": text}

if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=9000)
