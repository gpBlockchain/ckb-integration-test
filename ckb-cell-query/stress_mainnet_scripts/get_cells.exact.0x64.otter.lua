wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"

wrk.body = [[
{
    "id": 2,
    "jsonrpc": "2.0",
    "method": "get_cells",
    "params": [
        {
            "script_search_mode": "exact",
            "script": {
                "code_hash": "0x50bd8d6680b8b9cf98b73f3c08faf8b2a21914311954118ad6609be6e78a1b95",
                "hash_type": "data1",
                "args": "0x0a5c9645b871a28af6d8040ff43a8ea62a6f12a4c131c0bbdc3a20a0e46ff292"
            },
            "script_type": "type",
            "filter": {
                "block_range": [
                    "0x0",
                    "0xffffffffffffffff"
                ]
            }
        },
        "desc",
        "0x64",
        null
    ]
}
]]