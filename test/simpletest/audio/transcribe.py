import torch
import whisper

# GPU 使えるかチェック
device = "cuda" if torch.cuda.is_available() else "cpu"
print("device:", device)

# 3090 なら small / medium / large どれでもOK
# 最初は "small" くらいが速くてバランス良い
model = whisper.load_model("large", device=device)

# ここに音声ファイルパス
audio_path = "input_from_peer.wav"

result = model.transcribe(
    audio_path,
    language="ja",      # 日本語固定
    fp16=(device == "cuda")
)

print("----- text -----")
print(result["text"])
