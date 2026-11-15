# Ckb-Cell-Query

### prepare ckb-data

prepare data: 1w to 1024w
address	private key	cell

| Address                                                                                                             | Private Key                                                          | Cell |
|---------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------|------|
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqx8dhw9u34aw9exk0e9vg34aulha78ndysn9n9r5 | 0x11f732bf42f4b8891134de15258952d35f1912356de273e54f2a67d7568ce9dc | 1w   |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqyvjrauknt7de68sepfgd7kxxmrkwqpspqds6ehm | 0xd30b1c73f56f0f25151b063dade293d472a6ccb4a21cbdef000a31ee9d0056c2 | 2w   |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqx6cee2qzuyq09aqdq6fknxgj2jx88rwms7x9nsf | 0xcd1cb8ee534afdd2655d5954223431084d00e6de9a18755448c60dd59bb60522 | 4w   |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjq8vd5vqcgnha7p3d3pf3fpyjwhe3ev4t6g93nwdk | 0xaba5e4beb2a7f2e010712b0923d03257bd8a946e46fe5560ab48b18efe35c02d | 8w   |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqq7ydv0eemtjjs4k5pgfcrdj0ks9r44eqqz38r66 | 0x6d81d726c5cc49641a46f79ba7b1d8fdfb1ddffe1a474c92888c0d1a105e7044 | 16w  |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqqx4akxmcutl9zzjxmajt2np7eh0dzqwdqurdncw | 0x5f7ef23197d83910fa2f5a69cb352f6efa614eb01bf37113e62d691234f35b30 | 32w  |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqpsap7afv75dw732gwra7e5qhsxjdnfsvgq5pd9d | 0xcb4b255b716e548e619bc8262470c02fb2edb1a9801939375d1393d575a1b163 | 64w  |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqyngwy7qke3fdh9cpv5racs6hya2r3a2eqf03tqs | 0xf6ecbe94aa51b68c8650e913ad8986425f369ef0d3447e041bc373d85979f5e0 | 128w |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjq9qv25udpa0rdfzdp5256xlyxsmvd046vss42nvn | 0xdc39bb5b1ba457c0767abbf9f4f622848a2fbf737387c303c4379c79d1951d97 | 256w |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqpl7zprrew23axcrnp7vdjcemnp67ngamgyp60xr | 0x58dc3625d67a848ae4dd18b4322665f1307a06c4e0a98e3c38791d0ae6524460 | 512w |
| ckt1qpcf7076zt6krnavly3883t6nrlduxy28ud9nv0c3rg387wvuzryjqy4g259cx5jx4u8wvsxytvnjgudzwpjpwskrqdwc | 0x068b019ee0e2f5b9b0d927f7c7d3ee9804c28980b6bf907bba1ac5e6f5076d97 | 1024w|

- script
```angular2html


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 10000   --n-users 11

CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 10000   --n-users 10


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 20000   --n-users 9


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 40000   --n-users 8


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 80000   --n-users 7


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 160000   --n-users 6



CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 320000   --n-users 5


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 640000   --n-users 4


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 1280000   --n-users 3



CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 2560000   --n-users 2


CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 ./ckb-bench dispatch   --data-dir data/   --rpc-urls http://172.31.23.160:8020  --capacity-per-cell 7100000000   --cells-per-user 5120000   --n-users 1

```

## ckb-pool-test

Includes preparation for starting a CKB node.
- stress testing: A block is produced every 20 seconds. The CKB node is stress-tested for 1000 seconds, expecting the pool to reach a transaction count of 500,000.
- block production: Single-threaded block production is optimized for speed.

#### Interpretation of Test Results

| Key | Description |
|-----|-------------|
| ckb_version | Version of CKB |
| 0_50w_client_send_tps | Performance of the pool during stress testing |
| 50w_0_transactions_per_second | Performance during block production |
| average_block_time_ms | Block production latency |
| max_pending_size | Maximum block height during stress testing |
| min_pending_size | Minimum height after block production |
| grafana | Grafana statistics from stress testing to block production |
| tx_pool_0_50w_report | CKB-bench report during stress testing |
| tx_pool_50w_0_report | CKB-bench report during block production |


## ckb-cell-query

Using wkr to stress test the performance of querying with different numbers of cells, testing interfaces:

- get_cells_capacity
- get_cells

#### Test Parameters
| Parameter  | Description       |
|------------|-------------------|
| -t1        | Number of threads |
| -c1        | Number of connections |
| d5m        | Test for 5 minutes  |



#### Interpretation of Test Results

| Key               | Description                                                                                             |
|-------------------|---------------------------------------------------------------------------------------------------------|
| Script            | The Lua script used for the stress test.                                                                |
| Test Duration     | The duration of each stress test.                                                                       |
| Target URL        | The URL of the server being tested.                                                                     |
| Threads           | The number of threads used in the stress test.                                                          |
| Connections       | The number of connections used in the stress test.                                                      |
| Requests/Sec      | The average number of requests per second sent during the stress test.                                   |
| Latency 50%       | The median (50th percentile) response latency, indicating the response time for half of the requests.  |
| Latency 75%       | The 75th percentile response latency, indicating the response time for 75% of the requests.            |
| Latency 90%       | The 90th percentile response latency, indicating the response time for 90% of the requests.            |
| Latency 99%       | The 99th percentile response latency, indicating the response time for 99% of the requests.            |
| Total Requests    | The total number of requests sent during the stress test.                                               |