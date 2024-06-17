wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"

wrk.body = [[
{
    "id": 2,
    "jsonrpc": "2.0",
    "method": "get_cells",
    "params": [
        {
            "script_search_mode": "prefix",
            "script": {
                "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
                "hash_type": "type",
                "args": "0x"
            },
            "script_type": "lock",
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
--  query result is empty