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
                "code_hash": "0x50bd8d6680b8b9cf98b73f3c08faf8b2a21914311954118ad6609be6e78a1b95",
                "hash_type": "data1",
                "args": "0x"
            },
            "script_type": "type",
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