use anyhow::{anyhow, Result};
use hound::WavWriter;
use log::info;
use reqwest::{multipart, Client};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout};
use aws_config::meta::region::RegionProviderChain;
use aws_config::{BehaviorVersion, SdkConfig};
use aws_sdk_s3 as s3;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_transcribe as transcribe;
use serde_json::Value;

#[derive(Clone)]
struct Config {
    sip_bind_ip: String,
    sip_port: u16,
    rtp_port: u16,
    local_ip: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let sip_bind_ip =
            std::env::var("SIP_BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port = std::env::var("SIP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5060);
        let rtp_port = std::env::var("RTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(40000);
        let local_ip = std::env::var("LOCAL_IP").unwrap_or_else(|_| "127.0.0.1".to_string());

        Ok(Self {
            sip_bind_ip,
            sip_port,
            rtp_port,
            local_ip,
        })
    }
}

// Ollama /api/chat 用の型
#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaMessage>,
    done: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentOut,
}

#[derive(Deserialize)]
struct GeminiContentOut {
    parts: Vec<GeminiPart>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cfg = Config::from_env()?;

    let bind_addr = format!("{}:{}", cfg.sip_bind_ip, cfg.sip_port);
    let sip_socket = UdpSocket::bind(&bind_addr).await?;
    info!("SIP UAS listening on {}", bind_addr);

    let mut buf = [0u8; 2048];

    loop {
        let (len, src) = sip_socket.recv_from(&mut buf).await?;
        let msg = String::from_utf8_lossy(&buf[..len]).to_string();

        // ここでは1コールずつ素直に処理する（spawnしない）
        if let Err(e) = handle_sip_message(&cfg, &sip_socket, src, msg).await {
            eprintln!("handle_sip_message error: {e:?}");
        }
    }
}

async fn handle_sip_message(
    cfg: &Config,
    socket: &UdpSocket,
    src: SocketAddr,
    msg: String,
) -> Result<()> {
    if msg.starts_with("INVITE ") {
        info!("Received INVITE from {src}");
        let (via, from, to, call_id, cseq) = parse_basic_headers(&msg)?;
        let (remote_rtp_addr, _remote_payload_type) = parse_sdp_remote_rtp(&msg)?;

        // 1) 100 Trying
        let trying = build_100_trying(&via, &from, &to, &call_id, &cseq);
        socket.send_to(trying.as_bytes(), src).await?;
        info!("Sent 100 Trying to {src}");

        // 2) 180 Ringing
        let ringing = build_180_ringing(&via, &from, &to, &call_id, &cseq);
        socket.send_to(ringing.as_bytes(), src).await?;
        info!("Sent 180 Ringing to {src}");

        // 3) SDP 付き 200 OK
        let sdp = format!(
            concat!(
                "v=0\r\n",
                "o=rustbot 1 1 IN IP4 {ip}\r\n",
                "s=Rust PCMU Bot\r\n",
                "c=IN IP4 {ip}\r\n",
                "t=0 0\r\n",
                "m=audio {rtp} RTP/AVP 0\r\n",
                "a=rtpmap:0 PCMU/8000\r\n",
            ),
            ip = cfg.local_ip,
            rtp = cfg.rtp_port,
        );

        let resp = build_200_ok(&via, &from, &to, &call_id, &cseq, cfg, &sdp);
        socket.send_to(resp.as_bytes(), src).await?;
        info!("Sent 200 OK to {src}");

        // ACK を待つ（この間は次のパケットを受け付けない＝単一通話前提）
        let wait_result = timeout(Duration::from_secs(5), async {
            let mut buf = [0u8; 2048];
            loop {
                let (len, ack_src) = socket.recv_from(&mut buf).await?;
                let ack_msg = String::from_utf8_lossy(&buf[..len]);
                if ack_msg.starts_with("ACK ") {
                    // Zoiper などは REGISTER/INVITE で送信元ポートが変わることがあるので、
                    // IP と Call-ID で ACK を判定する
                    let same_peer = ack_src.ip() == src.ip();
                    let same_call = ack_msg.contains(&call_id);
                    if same_peer && same_call {
                        info!("Received ACK from {ack_src}, start RTP to {remote_rtp_addr}");
                        return Ok::<(), std::io::Error>(());
                    } else {
                        info!(
                            "Ignored ACK from {ack_src} (same_peer={same_peer} same_call={same_call})"
                        );
                    }
                } else {
                    info!(
                        "Non-ACK while waiting (from {ack_src}): {}",
                        ack_msg.lines().next().unwrap_or_default()
                    );
                }
            }
        })
        .await;

        match wait_result {
            Ok(Ok(())) => {
                // 録音（相手のPCMUをWAVに保存）
                let recv_port = cfg.rtp_port; // SDPで名乗ってるポートと揃える
                let out_path = "test/simpletest/audio/input_from_peer.wav".to_string();
                if let Err(e) = recv_rtp_to_wav(recv_port, &out_path, 10).await {
                    log::error!("RTP recv error: {e:?}");
                    return Ok(());
                }

                //let out_path_tmp = "test/simpletest/audio/test_karaage.wav".to_string();
                let transcribed_text = match transcribe_and_log(&out_path).await {
                    Ok(text) => text,
                    Err(e) => {
                        log::error!("transcribe_and_log error: {e:?}");
                        return Ok(());
                    }
                };

                let answer_wav_path = match handle_user_question_from_whisper(&transcribed_text).await {
                    Ok(path) => path,
                    Err(e) => {
                        log::error!("handle_user_question_from_whisper error: {e:?}");
                        return Ok(());
                    }
                };

                let send_local = SocketAddr::new("0.0.0.0".parse().unwrap(), 0);
                if let Err(e) =
                    send_fixed_pcmu(send_local, remote_rtp_addr, Some(&answer_wav_path)).await
                {
                    log::error!("RTP send error: {e:?}");
                }

            }
            Ok(Err(e)) => {
                eprintln!("Error while waiting ACK: {e:?}");
            }
            Err(_) => {
                eprintln!("ACK timeout from {src}, won't send RTP");
            }
            
        }
        
    } else if msg.starts_with("BYE ") {
        info!("Received BYE from {src}");

        let (via, from, to, call_id, cseq) = parse_basic_headers(&msg)?;
        let resp = build_200_ok_simple(&via, &from, &to, &call_id, &cseq);

        socket.send_to(resp.as_bytes(), src).await?;
        info!("Sent 200 OK for BYE to {src}");
    } else {

        info!(
            "Received non-INVITE: first line = {}",
            msg.lines().next().unwrap_or("")
        );
    }

    Ok(())
}

fn parse_basic_headers(msg: &str) -> Result<(String, String, String, String, String)> {
    let mut via = String::new();
    let mut from = String::new();
    let mut to = String::new();
    let mut call_id = String::new();
    let mut cseq = String::new();

    for line in msg.lines() {
        let line = line.trim_end();
        if line.starts_with("Via:") {
            via = line.to_string();
        } else if line.starts_with("From:") {
            from = line.to_string();
        } else if line.starts_with("To:") {
            to = line.to_string();
        } else if line.starts_with("Call-ID:") {
            call_id = line.to_string();
        } else if line.starts_with("CSeq:") {
            cseq = line.to_string();
        }
    }

    if via.is_empty() || from.is_empty() || to.is_empty() || call_id.is_empty() || cseq.is_empty()
    {
        anyhow::bail!("missing SIP headers");
    }
    Ok((via, from, to, call_id, cseq))
}

fn parse_sdp_remote_rtp(msg: &str) -> Result<(SocketAddr, u8)> {
    let parts: Vec<&str> = msg.split("\r\n\r\n").collect();
    if parts.len() < 2 {
        anyhow::bail!("no SDP body");
    }
    let sdp = parts[1];
    let mut ip = None;
    let mut port = None;
    let mut pt = 0u8;

    for line in sdp.lines() {
        let line = line.trim();
        if line.starts_with("c=IN IP4 ") {
            let v = line.trim_start_matches("c=IN IP4 ").trim();
            ip = Some(v.to_string());
        } else if line.starts_with("m=audio ") {
            // m=audio <port> RTP/AVP <pt>
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 4 {
                port = cols[1].parse::<u16>().ok();
                pt = cols[3].parse::<u8>().unwrap_or(0);
            }
        }
    }

    let ip = ip.ok_or_else(|| anyhow::anyhow!("no c=IN IP4 in SDP"))?;
    let port = port.ok_or_else(|| anyhow::anyhow!("no m=audio in SDP"))?;
    let addr = format!("{ip}:{port}").parse()?;
    Ok((addr, pt))
}

fn build_100_trying(
    via: &str,
    from: &str,
    to: &str,
    call_id: &str,
    cseq: &str,
) -> String {
    format!(
        "SIP/2.0 100 Trying\r\n\
{via}\r\n\
{from}\r\n\
{to}\r\n\
{call_id}\r\n\
{cseq}\r\n\
Content-Length: 0\r\n\
\r\n",
        via = via,
        from = from,
        to = to,
        call_id = call_id,
        cseq = cseq,
    )
}

fn build_180_ringing(
    via: &str,
    from: &str,
    to: &str,
    call_id: &str,
    cseq: &str,
) -> String {
    format!(
        "SIP/2.0 180 Ringing\r\n\
{via}\r\n\
{from}\r\n\
{to}\r\n\
{call_id}\r\n\
{cseq}\r\n\
Content-Length: 0\r\n\
\r\n",
        via = via,
        from = from,
        to = to,
        call_id = call_id,
        cseq = cseq,
    )
}


fn build_200_ok(
    via: &str,
    from: &str,
    to: &str,
    call_id: &str,
    cseq: &str,
    cfg: &Config,
    sdp: &str,
) -> String {
    let content_length = sdp.as_bytes().len();
    format!(
        "SIP/2.0 200 OK\r\n\
{via}\r\n\
{from}\r\n\
{to};tag=rustbot\r\n\
{call_id}\r\n\
{cseq}\r\n\
Contact: <sip:rustbot@{ip}:{port}>\r\n\
Content-Type: application/sdp\r\n\
Content-Length: {len}\r\n\
\r\n\
{sdp}",
        via = via,
        from = from,
        to = to,
        call_id = call_id,
        cseq = cseq,
        ip = cfg.local_ip,
        port = cfg.sip_port,
        len = content_length,
        sdp = sdp
    )
}

fn build_200_ok_simple(
    via: &str,
    from: &str,
    to: &str,
    call_id: &str,
    cseq: &str,
) -> String {
    format!(
        "SIP/2.0 200 OK\r\n\
{via}\r\n\
{from}\r\n\
{to}\r\n\
{call_id}\r\n\
{cseq}\r\n\
Content-Length: 0\r\n\
\r\n",
        via = via,
        from = from,
        to = to,
        call_id = call_id,
        cseq = cseq,
    )
}


/// 超雑なRTPパケット。PCMU(payload_type=0)専用。
fn build_rtp_packet(seq: u16, ts: u32, ssrc: u32, payload: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; 12 + payload.len()];
    // Header
    buf[0] = (2u8 << 6) | 0; // V=2, P=0, X=0, CC=0
    buf[1] = 0; // M=0, PT=0(PCMU)
    buf[2] = (seq >> 8) as u8;
    buf[3] = (seq & 0xff) as u8;
    buf[4] = (ts >> 24) as u8;
    buf[5] = (ts >> 16) as u8;
    buf[6] = (ts >> 8) as u8;
    buf[7] = (ts & 0xff) as u8;
    buf[8] = (ssrc >> 24) as u8;
    buf[9] = (ssrc >> 16) as u8;
    buf[10] = (ssrc >> 8) as u8;
    buf[11] = (ssrc & 0xff) as u8;
    buf[12..].copy_from_slice(payload);
    buf
}

/// 16bit PCM (リトルエンディアン, -32768..32767) を μ-law(PCMU) 1byte に変換
fn linear16_to_mulaw(sample: i16) -> u8 {
    const BIAS: i16 = 0x84;
    const CLIP: i16 = 32635;

    let mut s = sample;
    let mut sign = 0u8;

    if s < 0 {
        s = -s;
        sign = 0x80;
    }

    if s > CLIP {
        s = CLIP;
    }

    s += BIAS;

    // セグメント番号を計算
    let mut segment: u8 = 0;
    let mut value = (s as u16) >> 7; // 上位ビットから見ていく
    while value > 0 {
        segment += 1;
        value >>= 1;
        if segment >= 8 {
            break;
        }
    }

    let mantissa = ((s >> (segment + 3)) & 0x0F) as u8;
    let mu = !(sign | (segment << 4) | mantissa);
    mu
}

/// WAV(16bit, mono) を読み込んで、20ms(160サンプル@8kHz相当)ごとの PCMU フレームにする
fn load_wav_as_pcmu_frames(path: &str) -> Result<Vec<Vec<u8>>> {
    use hound::WavReader;

    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    if spec.channels != 1 {
        anyhow::bail!("WAV must be mono (1ch), got {} ch", spec.channels);
    }
    if spec.bits_per_sample != 16 {
        anyhow::bail!("WAV must be 16-bit PCM, got {} bits", spec.bits_per_sample);
    }

    // まず全部 16bit サンプルとして読む
    let mut samples: Vec<i16> = Vec::new();
    for s in reader.samples::<i16>() {
        samples.push(s?);
    }

    // サンプリングレートに応じて 8kHz 相当の列に変換
    let base_samples: Vec<i16> = match spec.sample_rate {
        8000 => {
            // そのまま
            samples
        }
        24000 => {
            // 超雑な 24kHz → 8kHz ダウンサンプリング: 3サンプルに1つを取る
            samples.iter().step_by(3).copied().collect()
        }
        other => {
            anyhow::bail!("WAV must be 8000 Hz or 24000 Hz, got {}", other);
        }
    };

    // 20ms (160サンプル @8kHz) ごとに PCMU フレームに切る
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut current: Vec<u8> = Vec::with_capacity(160);

    for s in base_samples {
        let mu = linear16_to_mulaw(s);
        current.push(mu);

        if current.len() == 160 {
            frames.push(current.clone());
            current.clear();
        }
    }

    // 端数があればパディングして1フレームにする
    if !current.is_empty() {
        while current.len() < 160 {
            current.push(0xFF); // 適当な静音っぽい値
        }
        frames.push(current);
    }

    Ok(frames)
}

/// WAVファイルをPCMUに変換してRTP送信する
async fn send_fixed_pcmu(
    local: SocketAddr,
    remote: SocketAddr,
    wav_override: Option<&str>,
) -> Result<()> {
    // WAVパスを指定できなければ環境変数から取る
    let wav_path = wav_override
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            std::env::var("PCM_WAV_PATH")
                .unwrap_or_else(|_| "test/simpletest/audio/test.wav".to_string())
        });

    let frames = load_wav_as_pcmu_frames(&wav_path)?;
    if frames.is_empty() {
        anyhow::bail!("no frames in wav file");
    }

    let socket = UdpSocket::bind(local).await?;
    let mut seq: u16 = 0;
    let mut ts: u32 = 0;
    let ssrc: u32 = 0x12345678;

    log::info!(
        "RTP sending WAV {} from {} to {}, frames={}",
        wav_path,
        local,
        remote,
        frames.len()
    );

    // 今はとりあえず1回分だけ再生（ループしたければ for _ in 0..N とかにしてもOK）
    for frame in &frames {
        // frame.len() は 160 のはず（最後のフレームもパディング済み）
        let pkt = build_rtp_packet(seq, ts, ssrc, frame);
        socket.send_to(&pkt, remote).await?;

        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(frame.len() as u32); // 1サンプル=1カウント @8kHz

        sleep(Duration::from_millis(20)).await;
    }

    log::info!("RTP WAV sending finished");
    Ok(())
}

/// μ-law(PCMU, 8bit) → 16bit PCM 変換
fn mulaw_to_linear16(mu: u8) -> i16 {
    const BIAS: i16 = 0x84;

    // ビット反転
    let mu = !mu;
    let sign = (mu & 0x80) != 0;
    let segment = (mu & 0x70) >> 4;
    let mantissa = mu & 0x0F;

    // G.711 μ-law 復号
    let mut value = ((mantissa as i16) << 4) + 0x08;
    value <<= segment as i16;
    value -= BIAS;

    if sign { -value } else { value }
}

/// RTP(PCMU)を受信して、WAV(16bit, 8kHz, mono)として保存する
async fn recv_rtp_to_wav(
    listen_port: u16,
    out_path: &str,
    max_duration_secs: u64,
) -> Result<()> {
    use tokio::time::{timeout, Duration};

    let local = SocketAddr::new("0.0.0.0".parse().unwrap(), listen_port);
    let socket = UdpSocket::bind(local).await?;
    log::info!("RTP recv socket bound on {}", local);

    let mut buf = [0u8; 2048];
    let mut samples: Vec<i16> = Vec::new();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(max_duration_secs);

    loop {
        let remain = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remain.is_zero() {
            break;
        }

        // 残り時間以内にパケットが来るのを待つ
        let res = timeout(remain, socket.recv_from(&mut buf)).await;
        let (len, _src) = match res {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                log::warn!("RTP recv error: {e}");
                continue;
            }
            Err(_) => {
                // timeout
                break;
            }
        };

        if len <= 12 {
            continue; // RTPヘッダだけ/壊れたパケット
        }

        // RTPヘッダ(12byte)を飛ばしてペイロードだけ見る
        let payload = &buf[12..len];

        for &b in payload {
            let s = mulaw_to_linear16(b);
            samples.push(s);
        }
    }

    log::info!(
        "RTP recv finished, {} samples -> writing WAV to {}",
        samples.len(),
        out_path
    );

    // WAVとして保存
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 8000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(out_path, spec)?;
    for s in samples {
        writer.write_sample(s)?;
    }
    writer.finalize()?;

    log::info!("WAV written: {}", out_path);
    Ok(())
}


#[derive(Deserialize)]
struct WhisperResponse {
    text: String,
}

async fn transcribe_and_log(wav_path: &str) -> Result<String> {
    if aws_transcribe_enabled() {
        match transcribe_with_aws(wav_path).await {
            Ok(text) => {
                if text.trim().is_empty() {
                    log::warn!(
                        "AWS Transcribe returned empty text, falling back to local Whisper."
                    );
                } else {
                    info!("User question (aws): {}", text);
                    return Ok(text);
                }
            }
            Err(e) => {
                log::error!("AWS Transcribe failed: {e:?}. Falling back to local Whisper.");
            }
        }
    }

    let client = Client::new();

    // ファイル読み込み
    let bytes = tokio::fs::read(wav_path).await?;

    let part = multipart::Part::bytes(bytes)
        .file_name("question.wav")
        .mime_str("audio/wav")?;

    let form = multipart::Form::new().part("file", part);

    // Whisperサーバに投げる
    let resp = client
        .post("http://localhost:9000/transcribe")
        .multipart(form)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("whisper error: {} - {}", status, body);
    }

    let result: WhisperResponse = resp.json().await?;

    // ★ ここで info で出す
    let text = result.text;
    info!("User question (whisper): {}", text);

    Ok(text)
}

fn aws_transcribe_enabled() -> bool {
    std::env::var("USE_AWS_TRANSCRIBE")
        .map(|v| {
            let lower = v.to_ascii_lowercase();
            lower == "1" || lower == "true" || lower == "yes"
        })
        .unwrap_or(false)
}

fn build_llm_prompt(user_text: &str) -> String {
    format!(
        "以下の質問に「はい」または「いいえ」で回答し、回答全体を30文字以内にまとめてください。質問: {}",
        user_text
    )
}

fn prepare_wav_for_transcribe(wav_path: &str) -> Result<Vec<u8>> {
    const TARGET_RATE: u32 = 16_000;

    let mut reader = hound::WavReader::open(wav_path)?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.bits_per_sample != 16 {
        anyhow::bail!(
            "Expected mono 16-bit WAV for AWS Transcribe, got {} ch / {} bits",
            spec.channels,
            spec.bits_per_sample
        );
    }

    if spec.sample_rate == TARGET_RATE {
        return Ok(fs::read(wav_path)?);
    }

    let mut samples: Vec<i16> = Vec::new();
    for s in reader.samples::<i16>() {
        samples.push(s?);
    }

    let mut new_spec = spec;
    new_spec.sample_rate = TARGET_RATE;

    let mut output: Vec<i16> = Vec::new();
    if spec.sample_rate == 8_000 {
        output.reserve(samples.len() * 2);
        for sample in samples {
            output.push(sample);
            output.push(sample);
        }
    } else {
        log::warn!(
            "Unexpected WAV sample rate {} Hz, sending original file to AWS Transcribe",
            spec.sample_rate
        );
        drop(reader);
        return Ok(fs::read(wav_path)?);
    }

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, new_spec)?;
        for sample in output {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

// Whisperで文字起こしされたテキストを受け取って呼ぶ関数
pub async fn handle_user_question_from_whisper(text: &str) -> Result<String> {
    info!("User question (whisper): {}", text);
    let llm_prompt = build_llm_prompt(text);

    let answer = match call_gemini(&llm_prompt).await {
        Ok(ans) => {
            info!("LLM answer (gemini): {}", ans);
            ans
        }
        Err(gemini_err) => {
            log::error!("call_gemini failed: {gemini_err:?}, falling back to ollama");
            match call_ollama(&llm_prompt).await {
                Ok(fallback) => {
                    info!("LLM answer (ollama fallback): {}", fallback);
                    fallback
                }
                Err(ollama_err) => {
                    log::error!(
                        "call_ollama also failed: {ollama_err:?}. Using default apology message."
                    );
                    "すみません、うまく答えを用意できませんでした。".to_string()
                }
            }
        }
    };

    let answer_wav = "test/simpletest/audio/ollama_answer.wav";
    synth_zundamon_wav(&answer, answer_wav).await?;

    // あとでここで TTS → RTP 送信とかにも繋げられる
    Ok(answer_wav.to_string())
}

async fn call_ollama(question: &str) -> Result<String> {
    let client = Client::new();

    let req = OllamaChatRequest {
        model: "gemma3:4b".to_string(), // ← ここだけ修正
        messages: vec![OllamaMessage {
            role: "user".to_string(),
            content: question.to_string(),
        }],
        stream: false,
    };

    let resp = client
        .post("http://localhost:11434/api/chat")
        .json(&req)
        .send()
        .await?;

    let status = resp.status();
    let body_text = resp.text().await?;

    // ★ まずは全部ログる
    info!("Ollama status: {}", status);
    info!("Ollama raw body: {}", body_text);

    if !status.is_success() {
        anyhow::bail!("Ollama HTTP error {}: {}", status, body_text);
    }

    // ここで初めて JSON としてパース
    #[derive(Deserialize)]
    struct ChatResponse {
        message: Option<OllamaMessage>,
        // 他のフィールドは無視してOK
    }

    let body: ChatResponse = serde_json::from_str(&body_text)?;

    let answer = body
        .message
        .map(|m| m.content)
        .unwrap_or_else(|| "<no response>".to_string());

    Ok(answer)
}

pub async fn synth_zundamon_wav(text: &str, out_path: &str) -> Result<()> {
    let client = Client::new();
    let speaker_id = 3; // ずんだもん ノーマル

    // 1. audio_query
    let query_resp = client
        .post("http://localhost:50021/audio_query")
        .query(&[("text", text), ("speaker", &speaker_id.to_string())])
        .send()
        .await?;

    let status = query_resp.status();
    let query_body = query_resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("audio_query error {}: {}", status, query_body);
    }

    // 2. synthesis
    let synth_resp = client
        .post("http://localhost:50021/synthesis")
        .query(&[("speaker", &speaker_id.to_string())])
        .header("Content-Type", "application/json")
        .body(query_body)
        .send()
        .await?;

    let status = synth_resp.status();
    let wav_bytes = synth_resp.bytes().await?;
    if !status.is_success() {
        anyhow::bail!("synthesis error {} ({} bytes)", status, wav_bytes.len());
    }

    // 3. WAV保存
    tokio::fs::write(out_path, &wav_bytes).await?;
    info!("Zundamon TTS written to {}", out_path);

    Ok(())
}

async fn call_gemini(question: &str) -> Result<String> {
    let client = Client::new();

    // ★ APIキーは環境変数から読む
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY must be set");

    // 環境変数がなければ gemini-2.5-flash-lite を使う
    let model = std::env::var("GEMINI_MODEL")
        .unwrap_or_else(|_| "gemini-2.5-flash-lite".to_string());

    // ★ v1 + /models/{model}:generateContent
    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/{}:generateContent?key={}",
        model, api_key
    );

    let req_body = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: question.to_string(),
            }],
        }],
    };

    let resp = client.post(&url).json(&req_body).send().await?;
    let status = resp.status();
    let body_text = resp.text().await?;

    info!("Gemini status: {}", status);
    info!("Gemini raw body: {}", body_text);

    if !status.is_success() {
        anyhow::bail!("Gemini HTTP error {}: {}", status, body_text);
    }

    let body: GeminiResponse = serde_json::from_str(&body_text)?;

    let answer = body
        .candidates
        .as_ref()
        .and_then(|cands| cands.get(0))
        .and_then(|cand| cand.content.parts.get(0))
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "<no response>".to_string());

    Ok(answer)
}

async fn transcribe_with_aws(wav_path: &str) -> Result<String> {
    let bucket = std::env::var("AWS_TRANSCRIBE_BUCKET")
        .map_err(|_| anyhow!("AWS_TRANSCRIBE_BUCKET must be set when USE_AWS_TRANSCRIBE=1"))?;
    let prefix = std::env::var("AWS_TRANSCRIBE_PREFIX").unwrap_or_else(|_| "voicebot".to_string());

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis();
    let job_name = format!("voicebot-{}", timestamp);

    let normalized_prefix = if prefix.is_empty() {
        String::new()
    } else if prefix.ends_with('/') {
        prefix
    } else {
        format!("{}/", prefix)
    };
    let object_key = format!("{}{}.wav", normalized_prefix, job_name);

    let region_provider = RegionProviderChain::default_provider().or_default_provider();
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let wav_bytes = prepare_wav_for_transcribe(wav_path)?;
    let body_stream = ByteStream::from(wav_bytes);
    let s3_client = s3::Client::new(&config);
    info!("Uploading audio to s3://{}/{}", bucket, object_key);
    s3_client
        .put_object()
        .bucket(&bucket)
        .key(&object_key)
        .body(body_stream)
        .content_type("audio/wav")
        .send()
        .await?;

    let s3_uri = format!("s3://{}/{}", bucket, object_key);
    transcribe_with_aws_job(&config, &s3_uri, &job_name).await
}

async fn transcribe_with_aws_job(config: &SdkConfig, s3_uri: &str, job_name: &str) -> Result<String> {
    let client = transcribe::Client::new(config);

    let media = transcribe::types::Media::builder()
        .media_file_uri(s3_uri)
        .build();

    client
        .start_transcription_job()
        .transcription_job_name(job_name)
        .language_code(transcribe::types::LanguageCode::JaJp)
        .media(media)
        .media_format(transcribe::types::MediaFormat::Wav)
        .send()
        .await?;

    loop {
        let resp = client
            .get_transcription_job()
            .transcription_job_name(job_name)
            .send()
            .await?;

        if let Some(job) = resp.transcription_job() {
            use transcribe::types::TranscriptionJobStatus as Status;
            match job.transcription_job_status() {
                Some(Status::Completed) => {
                    if let Some(uri) = job
                        .transcript()
                        .and_then(|t| t.transcript_file_uri())
                    {
                        let resp = reqwest::get(uri).await?;
                        let body_text = resp.text().await?;

                        // ★ 追加：ここで AWS の JSON を全部ログに出す
                        log::info!("AWS transcript raw JSON: {}", body_text);

                        let transcript = parse_aws_transcript(&body_text)?;
                        return Ok(transcript);
                    } else {
                        anyhow::bail!("Transcribe job completed but transcript URI missing");
                    }
                }
                Some(Status::Failed) => {
                    anyhow::bail!("Transcribe job failed: {:?}", job.failure_reason());
                }
                _ => {
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
}

fn parse_aws_transcript(body_text: &str) -> Result<String> {
    let value: Value = serde_json::from_str(body_text)?;
    let transcript = value["results"]["transcripts"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("transcript"))
        .and_then(|node| node.as_str())
        .ok_or_else(|| anyhow!("Transcript JSON missing text"))?;
    Ok(transcript.to_string())
}
