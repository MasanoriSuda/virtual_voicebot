from scapy.all import IP, UDP, wrpcap
from scapy.layers.rtp import RTP
from scapy.layers.l2 import Ether
import soundfile as sf
import numpy as np

# ---------- 設定 ----------
wav_path = "karaage_shikei_question_org2.wav"   # ffmpeg で作った 8kHz PCM16 のWAV
pcap_path = "voice.pcap"     # 出力PCAP
dst_port = 4002              # SIPpシナリオの m=audio のポート
src_port = 4000              # 適当な送信元ポート
# --------------------------

# WAV 読み込み
data, sr = sf.read(wav_path)
if sr != 8000:
    raise ValueError(f"WAV must be 8000 Hz. Now: {sr} Hz")

# ステレオなら 1ch に
if len(data.shape) > 1:
    data = data[:, 0]

# 16bit PCM float(-1〜1) → μ-law(8bit)
def linear2ulaw(sample):
    MU = 255.0
    MAX = 32635.0
    s = np.clip(sample * 32768.0, -32768.0, 32767.0)
    sign = 0x80 if s < 0 else 0x00
    s = abs(s)
    mag = int(128 * np.log1p(s) / np.log1p(MAX))
    return (~(sign | mag)) & 0xFF

ulaw = np.array([linear2ulaw(x) for x in data], dtype=np.uint8)

pkts = []
timestamp = 0
ssrc = 0x12345678
payload_type = 0   # PCMU
seq = 0
frame_size = 160   # 20ms (8000Hz * 0.02)

for i in range(0, len(ulaw), frame_size):
    payload = bytes(ulaw[i:i+frame_size])
    if not payload:
        break

    rtp = RTP(
        version=2,
        payload_type=payload_type,
        sequence=seq,
        timestamp=timestamp,
        marker=0,
        sourcesync=ssrc
    ) / payload

    # ★ Ethernet ヘッダを付ける：これで link-type=EN10MB になる
    ether = Ether(src="00:11:22:33:44:55", dst="66:77:88:99:aa:bb")

    pkt = ether / IP(src="127.0.0.1", dst="127.0.0.1") / \
          UDP(sport=src_port, dport=dst_port) / \
          rtp

    pkts.append(pkt)

    seq += 1
    timestamp += frame_size

wrpcap(pcap_path, pkts)
print("Generated:", pcap_path, "packets:", len(pkts))
