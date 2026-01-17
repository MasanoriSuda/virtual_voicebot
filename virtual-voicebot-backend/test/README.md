# E2E / リグレッションランナー

## 実行
```
./run.sh            # always
./run.sh all
./run.sh custom
```

always/custom の対象は `test/plan/always.txt` / `test/plan/custom.txt` で管理する。
custom が空なら何も実行せず終了する。

## 生成物
- カテゴリごとに `result/<run_id>/` を作成する
  - 例: `test/sipp/sip/result/<run_id>/sip00001/`
  - JUnit: `test/sipp/sip/result/<run_id>/junit.xml`

## SIPp compose 実行の補助（単体）
```
docker compose -f test/docker-compose.sipp.yml up --build --abort-on-container-exit --exit-code-from sipp
docker compose -f test/docker-compose.sipp.yml down -v
```
起動待ちは sipp 側でリトライする方式（composeのhealthcheckには依存しない）。

## 失敗時の確認先
- 各ケースの `stdout.log` / `stderr.log`
- SIPp のログ（`messages.log` / `errors.log` / `trace_stat.csv`）
