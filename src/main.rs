use anyhow::Result;
use log::{info};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout};
use hound::WavWriter;
use tokio::task;
use tokio::process::Command;

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
            "v=0\r\n\
            o=rustbot 1 1 IN IP4 {ip}\r\n\
            s=Rust PCMU Bot\r\n\
            c=IN IP4 {ip}\r\n\
            t=0 0\r\n\
            m=audio {rtp} RTP/AVP 0\r\n\
            a=rtpmap:0 PCMU/8000\r\n",
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
                if ack_src == src && ack_msg.starts_with("ACK ") {
                    info!("Received ACK, start RTP to {remote_rtp_addr}");
                    return Ok::<(), std::io::Error>(());
                }
            }
        })
        .await;

        match wait_result {
            Ok(Ok(())) => {
                // ACK受信OK → 送信＆受信を並行タスクで動かす

                let send_local = SocketAddr::new("0.0.0.0".parse().unwrap(), 0);
                let send_remote = remote_rtp_addr;

                // 再生（ずんだもんWAVをPCMUで送る）
                let send_task = task::spawn(async move {
                    if let Err(e) = send_fixed_pcmu(send_local, send_remote).await {
                        log::error!("RTP send error: {e:?}");
                    }
                });

                // 録音（相手のPCMUをWAVに保存）
                let recv_port = cfg.rtp_port; // SDPで名乗ってるポートと揃える
                let out_path = "test/simpletest/audio/input_from_peer.wav".to_string();
                let recv_out_path = out_path.clone();
                let recv_task = task::spawn(async move {
                    if let Err(e) = recv_rtp_to_wav(recv_port, &recv_out_path, 10).await {
                        log::error!("RTP recv error: {e:?}");
                    }
                });

                // 受信完了を待つ
                if let Err(e) = recv_task.await {
                    log::error!("recv_task join error: {e:?}");
                }

                // ここで Whisper にかける
                match transcribe_with_whisper(&out_path).await {
                    Ok(text) => {
                        log::info!("Whisper ASR result: {}", text);
                        // ここで Ollama や Voicevox に渡す流れにつなげられる
                    }
                    Err(e) => {
                        log::error!("Whisper error: {e:?}");
                    }
                }

                // 必要なら待つ（今は待たずにOK、終了ログ見たいなら join! してもいい）
                // let _ = tokio::join!(send_task, recv_task);

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

async fn transcribe_with_whisper(audio_path: &str) -> anyhow::Result<String> {
    let output = Command::new("python3")
        .arg("/workspaces/virtual_voicebot/src/asr/whisper_transcribe.py")
        .arg(audio_path)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("whisper script failed: {stderr}"))
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    }
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
async fn send_fixed_pcmu(local: SocketAddr, remote: SocketAddr) -> Result<()> {
    // 環境変数からWAVパスを取る（なければデフォルト）
    let wav_path =
        std::env::var("PCM_WAV_PATH").unwrap_or_else(|_| "test/simpletest/audio/test.wav".to_string());

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
