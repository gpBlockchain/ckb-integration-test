# always_success + ckb-bench 压测全流程

本文记录如何在指定 CKB dev 链上：

1. 校验已经部署的 `always_success` 合约；
2. 生成 `always_success` lock 地址；
3. 从普通 secp256k1 地址向该地址充值；
4. 使用 `ckb-bench` 准备 100,000 个 Cell；
5. 执行目标 100 TPS、持续 60 秒的压测；
6. 核对链上报告。

## 1. 本次使用的参数

```bash
export RPC_URL="http://52.76.106.120:8334"
export CKB_BENCH_DIR="/Users/guopenglin/demo4/ckb-integration-test/ckb-bench"
export CKB_CLI="/Users/guopenglin/PycharmProjects/fiber-py-integration-test/source/ckb-cli"

export CONTRACT_TX_HASH="0xe62068dcd85431684f03268a93c93afd025c56c99ebab701eb1f0929e85e867d"
export CONTRACT_INDEX="0"
export LOCK_CODE_HASH="0x7c23037af09f76a736ba4f8e5de11e61c8f817432b9c1e32bf840a7e9a63291e"
export LOCK_HASH_TYPE="type"
export LOCK_ARG="0xc8328aabcd9b9e8e64fbc566c4385c3bdeb219d7"

export ALWAYS_SUCCESS_ADDRESS="ckt1qp7zxqm67z0hdfekhf8cuh0presu37qhgv4ec83jh7zq5l56vv53uqwgx292hnvmn68xf779vmzrshpmm6epn4c4kgeq6"
```

重要：部署交易的 output 0 带 Type ID，因此这里的
`LOCK_CODE_HASH` 是 Type ID Script hash，必须和
`LOCK_HASH_TYPE=type` 配套使用。不能改成 `data`、`data1` 或
`data2`。

## 2. 构建 ckb-bench

```bash
cd "$CKB_BENCH_DIR"
cargo build
```

确认二进制：

```bash
./target/debug/ckb-bench --help
```

## 3. 校验合约部署交易

```bash
"$CKB_CLI" \
  --url "$RPC_URL" \
  --output-format json \
  rpc get_transaction \
  --hash "$CONTRACT_TX_HASH"
```

需要确认：

- `tx_status.status` 是 `committed`；
- 合约 Cell 位于 output 0；
- output 0 有 Type ID type script；
- output 0 的 data 是 `always_success` 二进制。

本次部署交易确认在区块 `0x5c`。

## 4. 安全读取私钥

不要把真实私钥直接写进脚本、Markdown 或 shell 命令历史。

```bash
read -rsp "Private key: " PRIVATE_KEY
echo

# ckb-bench 需要不带 0x 的 64 位十六进制私钥。
export CKB_BENCH_OWNER_PRIVKEY="${PRIVATE_KEY#0x}"

# ckb-cli 通过权限为 600 的临时文件读取私钥。
export KEY_FILE
KEY_FILE="$(mktemp /private/tmp/ckb-bench-key.XXXXXX)"
chmod 600 "$KEY_FILE"
printf '%s\n' "$PRIVATE_KEY" > "$KEY_FILE"
trap 'rm -f "$KEY_FILE"' EXIT
```

可以用以下命令核对普通 secp 地址和 lock args：

```bash
"$CKB_CLI" \
  --local-only \
  --output-format json \
  util key-info \
  --privkey-path "$KEY_FILE"
```

本次账户的 lock args 是：

```text
0xc8328aabcd9b9e8e64fbc566c4385c3bdeb219d7
```

## 5. 生成并验证 always_success 地址

如果本机已安装 `@ckb-ccc/core`，可以从完整 Script 生成地址：

```bash
export ALWAYS_SUCCESS_ADDRESS="$(
  node -e '
    const { ccc } = require("@ckb-ccc/core");
    const address = ccc.Address.from({
      prefix: "ckt",
      script: {
        codeHash: process.env.LOCK_CODE_HASH,
        hashType: process.env.LOCK_HASH_TYPE,
        args: process.env.LOCK_ARG,
      },
    });
    process.stdout.write(address.toString());
  '
)"

echo "$ALWAYS_SUCCESS_ADDRESS"
```

使用 `ckb-cli` 反向解析，防止 code hash、hash type 或 args 配错：

```bash
"$CKB_CLI" \
  --local-only \
  --output-format json \
  util address-info \
  --address "$ALWAYS_SUCCESS_ADDRESS"
```

期望结果：

```json
{
  "lock_script": {
    "args": "0xc8328aabcd9b9e8e64fbc566c4385c3bdeb219d7",
    "code_hash": "0x7c23037af09f76a736ba4f8e5de11e61c8f817432b9c1e32bf840a7e9a63291e",
    "hash_type": "type"
  }
}
```

## 6. 给 always_success 地址充值

准备参数：

- 1,000 个派生用户；
- 每个用户 100 个 Cell；
- 总 Cell 数：100,000；
- 每个 Cell：100 CKB；
- 测试 Cell 总容量：10,000,000 CKB；
- 充值 10,100,000 CKB，保留手续费和找零空间。

执行充值：

```bash
"$CKB_CLI" \
  --url "$RPC_URL" \
  --output-format json \
  wallet transfer \
  --privkey-path "$KEY_FILE" \
  --to-address "$ALWAYS_SUCCESS_ADDRESS" \
  --capacity 10100000 \
  --fee-rate 1000 \
  --skip-check-to-address
```

记录输出的充值交易 hash，然后查询直到状态为 `committed`：

```bash
export FUND_TX_HASH="<wallet transfer 返回的 tx hash>"

"$CKB_CLI" \
  --url "$RPC_URL" \
  --output-format json \
  rpc get_transaction \
  --hash "$FUND_TX_HASH"
```

本次正确的 Type ID 地址充值交易：

```text
0xa3ae5a528d1bc109f70c1f132e7853f17aae13fb3b8b5be3774eafb868a84503
```

## 7. 准备 100,000 个 Cell

必须在充值交易 `committed` 且 indexer 已同步之后执行。

```bash
cd "$CKB_BENCH_DIR"

./target/debug/ckb-bench dispatch \
  --rpc-urls "$RPC_URL" \
  --n-users 1000 \
  --cells-per-user 100 \
  --capacity-per-cell 10000000000 \
  --data-dir /private/tmp/ckb-bench-100k \
  --lock-code-hash "$LOCK_CODE_HASH" \
  --lock-hash-type "$LOCK_HASH_TYPE" \
  --lock-dep-tx-hash "$CONTRACT_TX_HASH" \
  --lock-dep-index "$CONTRACT_INDEX" \
  --lock-dep-type code
```

说明：

- `capacity-per-cell` 使用 Shannon，`10000000000` 等于 100 CKB；
- `1000 × 100 = 100000` 个 Cell；
- 合约 output 0 直接作为 `code` 类型的 cell dep；
- `ckb-bench` 会生成约 100 笔、每笔最多 1,000 outputs 的拆分交易；
- 看到 `finished dispatch` 才表示全部拆分交易已经 committed。

本次拆分结果：

```text
100 transactions committed
100,000 test Cells
1 CKB total dispatch fee
```

## 8. 用 indexer 聚合核验

创建查询文件：

```bash
cat > /private/tmp/ckb-bench-always-success-search.json <<JSON
{
  "script": {
    "code_hash": "$LOCK_CODE_HASH",
    "hash_type": "$LOCK_HASH_TYPE",
    "args": "0x"
  },
  "script_type": "lock",
  "script_search_mode": "prefix"
}
JSON
```

查询所有派生 args 下的总容量：

```bash
"$CKB_CLI" \
  --url "$RPC_URL" \
  --output-format json \
  rpc get_cells_capacity \
  --json-path /private/tmp/ckb-bench-always-success-search.json
```

本次结果：

```json
{
  "block_number": 323,
  "capacity": "10099999.0"
}
```

容量构成：

```text
100,000 × 100 CKB + 99,999 CKB 最终找零 = 10,099,999 CKB
```

这和预期完全一致。

## 9. 执行目标 100 TPS 压测

本次参数：

- 目标发送 TPS：100；
- 压测时间：60 秒；
- 1 input / 1 output；
- 100 个并发 RPC 请求；
- 使用已经准备好的 always_success lock Cells。

```bash
cd "$CKB_BENCH_DIR"

./target/debug/ckb-bench bench \
  --rpc-urls "$RPC_URL" \
  --n-users 1000 \
  --n-inout 1 \
  --bench-time-ms 60000 \
  --tx-interval-ms 1 \
  --concurrent-requests 100 \
  --tps 100 \
  --lock-code-hash "$LOCK_CODE_HASH" \
  --lock-hash-type "$LOCK_HASH_TYPE" \
  --lock-dep-tx-hash "$CONTRACT_TX_HASH" \
  --lock-dep-index "$CONTRACT_INDEX" \
  --lock-dep-type code
```

发送阶段结束后，`ckb-bench` 还会等待 tx pool 清空以及零负载统计窗口，
所以命令不会在 60 秒整时立即退出。

## 10. 本次压测结果

```json
{
  "ckb_version": "0.208.0 (585395c 2026-07-14)",
  "set_send_tps": 100,
  "client_send_tps": 12,
  "transactions_per_second": 13,
  "total_transactions": 406,
  "total_transactions_size": 243580,
  "transactions_size_per_second": 7807,
  "average_block_time_ms": 3900,
  "average_block_transactions": 50,
  "average_block_transactions_size": 30447,
  "from_block_number": 358,
  "to_block_number": 365,
  "n_inout": 1,
  "n_nodes": 1
}
```

结论：

- 配置目标是 100 TPS；
- 客户端实际发送约 12 TPS；
- 链上统计约 13 TPS；
- 本次没有达到目标 100 TPS。

运行期间观察到远程 `send_transaction` 平均延迟约 0.7 秒。要继续逼近
100 TPS，应优先把 `ckb-bench` 部署到 RPC 节点同区域或同机运行，然后再
逐步增加 `--concurrent-requests`，同时监控 tx pool 和节点 CPU。

报告文件：

```text
$CKB_BENCH_DIR/report.json
$CKB_BENCH_DIR/report.html
```

## 11. 关键错误与排查

### `InvalidLength(66)`

原因：`CKB_BENCH_OWNER_PRIVKEY` 带了 `0x`。

修复：

```bash
export CKB_BENCH_OWNER_PRIVKEY="${PRIVATE_KEY#0x}"
```

### `ScriptNotFound`

典型错误：

```text
ScriptNotFound: code_hash: 0x7c2303...
```

原因：Type ID code hash 错配为 `data2`。

正确配置：

```bash
--lock-code-hash "$LOCK_CODE_HASH" \
--lock-hash-type type \
--lock-dep-tx-hash "$CONTRACT_TX_HASH" \
--lock-dep-index 0 \
--lock-dep-type code
```

### `ckb-bench info` 查询很慢

`info --n-users 1000` 会串行查询 1,000 个派生用户。远程 RPC 下可能需要
数分钟。批量核验优先使用第 8 节的 prefix `get_cells_capacity` 查询。

## 12. 清理私钥临时文件

命令退出时 `trap` 会自动删除。也可以主动执行：

```bash
rm -f "$KEY_FILE"
unset PRIVATE_KEY
unset CKB_BENCH_OWNER_PRIVKEY
```
