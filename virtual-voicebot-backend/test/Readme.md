このリポジトリではE2Eは test/ 配下に置く。
HTTP系のE2Eは Cargo.toml の [[test]] で登録しているため cargo test で実行される。
SIPp のE2Eは cargo test とは独立しており、詳細は `test/README.md` を参照する。
