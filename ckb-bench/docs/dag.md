


deploy dag contract 
```angular2html

```



prepare account
```angular2html
CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 \
./ckb-bench dispatch \
  --data-dir data/ \
  --rpc-urls http://127.0.0.1:8114 \
  --capacity-per-cell 1000000000000 \
  --cells-per-user 100 \
  --n-users 100
```

bench
```angular2html
CKB_BENCH_OWNER_PRIVKEY=98400f6a67af07025f5959af35ed653d649f745b8f54bf3f07bef9bd605ee946 \
  ./ckb-bench bench   --is-smoking-test    --rpc-urls http://127.0.0.1:8114   --bench-time-ms 300000    --n-users 100    --n-inout 1   --tx-interval-ms 10   --concurrent-requests 1   --add-tx-params tx.json

```

tx.json
```angular2html
{
  "deps": [
    {
      "out_point": {
        "tx_hash": dag_tx_hash,
        "index": "0x0"
      },
      "dep_type": "code"
    }
  ],
  "_type": {
    "code_hash": dag_code_hash,
    "hash_type": "type",
    "args": "0x78502aa68c984848c6693f6e92e71cd808644e1ebe516bb05a1764baf8025411"
  },
  "witness": "0x",
  "output_data": "0x",
  "min_fee": 100000,
  "max_fee": 100000
}

```

build tx
```angular2html
input: 
   lock: - 256 lock
   type: (empty or dag)
output:
    lock: - 256 lock
    type: dag
    data: dag:data
```