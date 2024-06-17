wrk.method = "POST"
wrk.headers["Content-Type"] = "application/json"

wrk.body = [[
{
    "id": 2,
    "jsonrpc": "2.0",
    "method": "get_cells_capacity",
    "params": [
        {
            "script_search_mode": "partial",
            "script": {
                "code_hash": "0x9b819793a64463aed77c615d6cb226eea5487ccfc0783043a587254cda2b6f26",
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
        }
    ]
}
]]
