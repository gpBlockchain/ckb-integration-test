wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"

wrk.body = [[
{
    "id": 2,
    "jsonrpc": "2.0",
    "method": "get_transactions",
    "params": [
        {
            "script_search_mode": "partial",
            "script": {
                "code_hash": "0x50bd8d6680b8b9cf98b73f3c08faf8b2a21914311954118ad6609be6e78a1b95",
                "hash_type": "data1",
                "args": "0x8a0d5884ef2b7bf7e6c06c32482015b4eb0985f9cc4112c12a20ca36716066c6"
            },
            "script_type": "type",
            "filter": {
                "block_range": [
                    "0xc36e31",
                    "0xc36e3d"
                ]
            }
        },
        "desc",
        "0x1",
        null
    ]
}
]]