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
                "code_hash": "0xd00c84f0ec8fd441c38bc3f87a371f547190f2fcff88e642bc5bf54b9e318323",
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
--  query result is empty