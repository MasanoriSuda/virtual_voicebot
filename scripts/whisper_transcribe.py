# scripts/whisper_transcribe.py
import sys
from faster_whisper import WhisperModel

def main():
    if len(sys.argv) < 2:
        print("")
        return

    audio_path = sys.argv[1]

    # モデルサイズは dev 用なら "base" "small" あたりでOK
    model = WhisperModel("base")  # 初回実行時にモデルをDL

    segments, info = model.transcribe(audio_path)
    text = "".join(seg.text for seg in segments)
    print(text.strip())

if __name__ == "__main__":
    main()

