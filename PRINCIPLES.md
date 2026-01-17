# Principles

1. Readability first  
   可読性・説明可能性を最優先する（“賢い”抽象化より、理解できる実装）。

2. Small, intentional changes  
   変更は最小差分・単一目的。目的外のリファクタや整形は別PR。

3. Safety and correctness over micro-optimizations  
   体感・推測で最適化しない。必要なら計測と根拠を添える。

4. Compatibility is a feature  
   公開API/設定/データ互換性は慎重に扱う。破壊的変更は設計から。

5. Tests are part of the change  
   変更にはテスト（新規 or 回帰）を含め、再現手順を残す。
