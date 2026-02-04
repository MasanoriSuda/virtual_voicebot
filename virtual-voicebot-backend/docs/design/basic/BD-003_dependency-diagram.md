# BD-003 付録：依存関係図

> 現状のモジュール依存を可視化（2026-02-03 時点）

## 1. 全体依存関係図

```mermaid
flowchart TB
    subgraph L0["Layer 0: Foundation（依存なし）"]
        entities["entities/"]
        config["config/"]
        error["error/"]
    end

    subgraph L1["Layer 1: Ports & Adapters"]
        ports["ports/"]
        recording["recording/"]
        notification["notification/"]
        db["db/"]
    end

    subgraph L2["Layer 2: Internal Services"]
        ai["ai/"]
        http["http/"]
        media["media/"]
        logging["logging/"]
    end

    subgraph L3["Layer 3: Protocol & Transport"]
        rtp["rtp/"]
        transport["transport/"]
        sip["sip/"]
    end

    subgraph L4["Layer 4: Session Orchestration"]
        session["session/"]
    end

    subgraph L5["Layer 5: Application Layer"]
        app["app/"]
    end

    subgraph L6["Layer 6: Entry Point"]
        main["main.rs"]
    end

    %% Layer 1 依存
    ports --> entities
    ports --> error
    recording --> ports
    notification --> ports
    db --> ports

    %% Layer 2 依存
    ai --> config
    ai --> error
    ai --> ports
    ai --> rtp
    http --> config
    http --> ports
    media --> recording
    media --> rtp
    logging --> config

    %% Layer 3 依存
    rtp --> config
    rtp --> session
    transport --> config
    transport --> rtp
    transport --> session
    sip --> config
    sip --> session
    sip --> transport

    %% Layer 4 依存
    session --> config
    session --> entities
    session --> media
    session --> ports
    session --> recording
    session --> rtp
    session --> sip
    session --> transport

    %% Layer 5 依存
    app --> config
    app --> ports
    app --> session

    %% Layer 6 依存
    main --> app
    main --> db
    main --> notification
    main --> ports
    main --> session
    main --> sip
    main --> transport
```

## 2. 許可される依存方向

```mermaid
flowchart LR
    subgraph 許可
        direction LR
        A1[app] --> B1[session]
        A2[session] --> B2[ports]
        A3[adapters] --> B3[ports]
        A4[ports] --> B4[entities]
        A5[infrastructure] --> B5[adapters]
    end
```

## 3. 禁止される依存方向

```mermaid
flowchart LR
    subgraph 禁止
        direction LR
        X1[session] -.->|❌| Y1[app]
        X2[session] -.->|❌| Y2[http]
        X3[session] -.->|❌| Y3[db]
        X4[entities] -.->|❌| Y4[外側モジュール]
        X5[ports] -.->|❌| Y5[adapters]
    end
```

## 4. 現状の検証結果（2026-02-03）

| 禁止依存 | 状態 |
|----------|------|
| session → app | ✅ なし |
| session → http | ✅ なし |
| session → db | ✅ なし |
| session → recording（直接） | ✅ ports経由に移行済み |
| app → db（直接） | ✅ ports経由 |
| entities → 外側 | ✅ なし |

## 5. モジュール層対応表

| モジュール | BD-003 レイヤー | 許可される依存先 |
|------------|----------------|-----------------|
| `entities/` | Enterprise Business Rules | なし（最内側） |
| `ports/` | Port定義 | entities, error |
| `app/` | Application Business Rules | ports, session, config |
| `session/` | Application Business Rules | ports, entities, config, protocol層 |
| `ai/`, `db/`, `http/`, `notification/` | Interface Adapters | ports, config, error |
| `sip/`, `rtp/`, `transport/` | Frameworks & Drivers | config, session（コールバック用） |
| `recording/`, `media/` | Interface Adapters | ports |
| `config/`, `error/`, `logging/` | Infrastructure | なし（横断的関心事） |

## 6. session モジュールの責務

session は **オーケストレーションハブ** として以下を統合：

```mermaid
flowchart TB
    session["session/coordinator"]

    subgraph Inputs
        sip_in["SIP Events"]
        rtp_in["RTP Frames"]
        timer_in["Timer Events"]
    end

    subgraph Outputs
        sip_out["SIP Commands"]
        rtp_out["RTP Tx"]
        app_out["App Events"]
        storage_out["Storage"]
        ingest_out["Ingest"]
    end

    subgraph Internal
        state["StateMachine"]
        handlers["Handlers"]
        services["Services"]
    end

    sip_in --> session
    rtp_in --> session
    timer_in --> session

    session --> state
    state --> handlers
    handlers --> services

    session --> sip_out
    session --> rtp_out
    session --> app_out
    session --> storage_out
    session --> ingest_out
```

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 作成者 |
|------|-----------|---------|--------|
| 2026-02-03 | 1.0 | 初版作成（#95 依存関係分析より） | Claude Code |
